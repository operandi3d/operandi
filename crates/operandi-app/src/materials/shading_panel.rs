//! The Shading tab: the scene's shader tree (ST-1,
//! specifications/shader-tree-2026-11-05). The tree and every edit live in
//! `operandi_core::shader_tree`; this file draws the panel and turns clicks
//! into `Command::ShaderTree` edits, so undo, save and the MCP bridge see
//! the same operations.
//!
//! Provisional in this slice: the Materials view and Filter on Selection
//! show the full tree; Library, Nodes, Bake Items and FX are empty fixed
//! rows; Lights mirrors the scene's lights by name only; nothing renders
//! from the tree yet (ST-4).
use super::*;
use crate::tree_list::{
    self, Cell, Column, DropAt, Heading, RowIcon, Selected, TreeEvent, TreeHooks, TreeRow, TreeSpec,
};
use crate::unavailable::Gate;
use operandi_core::scaffold::ChannelValue;
use operandi_core::shader_layer_menu::ShaderClipboard;
use operandi_core::shader_tree::{
    BlendMode, Channel, Drop as TreeDrop, ItemMask, NewLayer, Node, NodeId, NodeKind, OutputEffect,
    Projection, ShaderChannels, ShaderEffect, ShaderTreeEdit, TagMask, TagType, TextureEffect,
};
use operandi_core::texture_locator::{
    FalloffType, FrontCulling, RandomOffsets, TangentType, Tiling, uses_axis,
};
use std::collections::BTreeSet;
use theme::workspace as palette;

/// An item picker: "(none)" plus `list` (id, name); a missing id shows
/// "(missing)".
fn item_combo(ui: &mut Ui, salt: &str, slot: &mut Option<u64>, list: &[(u64, String)]) {
    let shown = slot.map_or("(none)".to_owned(), |id| {
        list.iter()
            .find(|(i, _)| *i == id)
            .map_or("(missing)".to_owned(), |(_, n)| n.clone())
    });
    ComboBox::from_id_salt(salt)
        .selected_text(shown)
        .show_ui(ui, |ui| {
            ui.selectable_value(slot, None, "(none)");
            for (id, name) in list {
                ui.selectable_value(slot, Some(*id), name);
            }
        });
}

const BLEND: f32 = 34.;
const PERCENT: f32 = 40.;
const EFFECT: f32 = 92.;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum ShadingView {
    #[default]
    ShaderTree,
    /// Stub: shows the same tree for now (ST-3).
    Materials,
}

/// The Filter dropdown (§4.1). Filtering only hides rows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum ShadingFilter {
    #[default]
    None,
    Material,
    Texture,
    Group,
    Shader,
    RenderOutput,
    FurMaterials,
    /// Stub: shows every row for now.
    OnSelection,
}
impl ShadingFilter {
    const ALL: [ShadingFilter; 8] = [
        Self::None,
        Self::Material,
        Self::Texture,
        Self::Group,
        Self::Shader,
        Self::RenderOutput,
        Self::FurMaterials,
        Self::OnSelection,
    ];
    fn label(self) -> &'static str {
        match self {
            Self::None => "(none)",
            Self::Material => "Material",
            Self::Texture => "Texture",
            Self::Group => "Group",
            Self::Shader => "Shader",
            Self::RenderOutput => "Render Output",
            Self::FurMaterials => "Fur Materials",
            Self::OnSelection => "Filter on Selection",
        }
    }
    fn shows(self, node: &Node) -> bool {
        match self {
            Self::None | Self::OnSelection => true,
            Self::Material => matches!(node.kind, NodeKind::Material(_)),
            Self::Texture => matches!(node.kind, NodeKind::Texture(_)),
            Self::Group => matches!(node.kind, NodeKind::Group(_) | NodeKind::Folder),
            Self::Shader => matches!(node.kind, NodeKind::Shader(_)),
            Self::RenderOutput => matches!(node.kind, NodeKind::RenderOutput(_)),
            // No fur materials exist yet.
            Self::FurMaterials => false,
        }
    }
}

/// A texture layer's Properties sub-tabs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum LayerTab {
    #[default]
    Layer,
    TextureLocator,
}

/// A row of the Shader Tree list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum ShaderKey {
    /// A fixed top-level row: Render, Library, Nodes, Lights,
    /// Environments, Bake Items, FX.
    Section(&'static str),
    Layer(NodeId),
    /// A texture layer's Texture Locator attribute row.
    Locator(NodeId),
    /// A light under the Lights section, by position.
    Light(usize),
    /// An Environment item under Environments (ENV §6), by item id.
    Environment(u64),
    /// Its Environment Material row (by the environment's item id).
    EnvironmentMaterial(u64),
    /// A texture layer under an environment (item id, layer id).
    EnvironmentLayer(u64, NodeId),
    /// A node graph under Nodes (NM-04), by its id.
    Graph(u64),
    /// A Material Group's "Nodes (N)" row: its free nodes (NE-11).
    Nodes(NodeId),
}

const COLUMNS: usize = 5;

/// UIC-152: the enable eye, Name, then blend mode, opacity and effect.
pub(super) fn shading_spec() -> TreeSpec {
    TreeSpec {
        id: "shader-tree",
        columns: vec![
            Column::state(
                Heading::Eye,
                "Enable · drag down to change several, Alt+click for only this one",
            ),
            Column::name(),
            Column::value("B", BLEND, "Blend mode"),
            Column::value("%", PERCENT, "Opacity"),
            Column::value("Effect", EFFECT, "What the layer drives"),
        ],
        own_search: false,
        last_fills: false,
    }
}

/// The custom value cells and right-click menus.
struct ShadingHooks<'a> {
    app: &'a mut Operandi,
}

impl TreeHooks<ShaderKey> for ShadingHooks<'_> {
    fn cell(&mut self, ui: &mut Ui, column: usize, key: ShaderKey, rect: Rect) {
        let ShaderKey::Layer(id) = key else {
            return;
        };
        let Some(node) = self.app.editor.scene.shader_tree.get(id).cloned() else {
            return;
        };
        match column {
            2 => self.app.blend_cell(ui, rect, &node),
            3 => self.app.opacity_cell(ui, rect, &node),
            4 => self.app.effect_cell(ui, rect, &node),
            _ => {}
        }
    }
    fn has_row_menu(&self, key: ShaderKey) -> bool {
        matches!(
            key,
            ShaderKey::Layer(_)
                | ShaderKey::Section("Render")
                | ShaderKey::Section("Environments")
                | ShaderKey::Environment(_)
                | ShaderKey::EnvironmentMaterial(_)
        )
    }
    fn row_menu(&mut self, ui: &mut Ui, key: ShaderKey, rename: &mut dyn FnMut()) {
        match key {
            ShaderKey::Layer(id) => self.app.layer_menu(ui, id, rename),
            ShaderKey::Environment(id) => self.app.environment_menu(ui, Some(id), rename),
            // EP §Save: the Environment Material's menu saves a preset.
            ShaderKey::EnvironmentMaterial(id) => {
                self.app.menu_owned_input = true;
                if ui.button("Save Environment Preset…").clicked() {
                    ui.close();
                    self.app.execute(&format!("preset.saveAs item:{id}"));
                }
            }
            ShaderKey::Section("Environments") => self.app.environment_menu(ui, None, rename),
            // PB-42: right-click the Render item to save its settings.
            ShaderKey::Section(_) => {
                self.app.menu_owned_input = true;
                if ui.button("Save Preset…").clicked() {
                    self.app
                        .request_save_preset(operandi_core::preset::PresetSource::RenderSettings);
                    ui.close();
                }
            }
            ShaderKey::Locator(_)
            | ShaderKey::Light(_)
            | ShaderKey::Graph(_)
            | ShaderKey::Nodes(_)
            | ShaderKey::EnvironmentLayer(..) => {}
        }
    }
}

/// Panel state that isn't part of the document (not undone, not saved).
#[derive(Default)]
pub(super) struct ShadingPanel {
    /// Texture rows whose locator row is shown (the small expander).
    pub(super) locator_open: BTreeSet<NodeId>,
    pub(super) layer_tab: LayerTab,
    /// Selected rows, in click order.
    pub(super) selected: Vec<NodeId>,
    anchor: Option<NodeId>,
    collapsed: BTreeSet<NodeId>,
    render_collapsed: bool,
    /// The Render row is selected (RI-02); it counts only while no layer
    /// or environment row is (`Operandi::render_item_selected`).
    pub(super) render_selected: bool,
    /// The undo gesture of a Render item field drag (one step per drag).
    pub(super) render_gesture: Option<u64>,
    /// Expanded section rows (Lights, Environments).
    pub(super) open_sections: BTreeSet<&'static str>,
    /// The selected Environment or Environment Material row (ENV §7); a
    /// layer selection clears it and it clears the layer selection.
    pub(super) environment: Option<crate::environment_ui::EnvironmentRow>,
    /// Environment rows closed with their triangle (open by default).
    environments_closed: BTreeSet<u64>,
    /// The Shift-click anchor among Environment rows (§3.3).
    pub(super) environment_anchor: Option<u64>,
    /// The list's view state (tree_list.rs): rename, drags, eye sweeps,
    /// search, column widths.
    pub(super) tree: Box<tree_list::TreeState<ShaderKey>>,
    /// An Alt+click "only this layer" and the layers it turned off.
    solo: Option<(NodeId, Vec<NodeId>)>,
    /// The layer whose % is being dragged, for one undo step per drag.
    opacity_drag: Option<NodeId>,
    /// The Gradient layer the Edit Gradient window edits (GR-Q03 v2).
    pub(super) gradient_window: Option<NodeId>,
    /// Curves the Edit Gradient window used before, newest first, for its
    /// Recent group (this session only, GR-Q09).
    pub(super) gradient_recent: Vec<operandi_core::gradient::Gradient>,
    /// Each layer row's cells from the last frame (tests and drags).
    pub(super) row_rects: std::collections::BTreeMap<NodeId, [Rect; 5]>,
    /// The Properties Name field while it is being typed in.
    name_draft: Option<(NodeId, String)>,
    pub(super) view: ShadingView,
    pub(super) filter: ShadingFilter,
    /// Operandi's own shader clipboard (layer menu §5).
    pub(super) clipboard: Option<Box<ShaderClipboard>>,
    /// Layers waiting on the delete confirmation (deleting a source takes
    /// its instances, layer menu §3).
    pub(super) confirm_delete: Option<Vec<NodeId>>,
    /// The selected graph row under Nodes (NM-04); a layer selection
    /// clears it.
    pub(super) graph: Option<u64>,
    /// The graph whose shared default is being dragged (one undo step).
    pub(super) graph_drag: Option<u64>,
}

impl ShadingPanel {
    pub(super) fn environments_closed_contains(&self, id: u64) -> bool {
        self.environments_closed.contains(&id)
    }
    /// Show environment `id`'s rows (section open, row expanded).
    pub(super) fn environments_open(&mut self, id: u64) {
        self.open_sections.insert("Environments");
        self.environments_closed.remove(&id);
    }
}

impl Operandi {
    /// The Shader Tree panel (UIC-152) on the shared tree list: two tool
    /// rows, then the Render trunk with the layers, then the fixed
    /// top-level sections (§2.1).
    pub(super) fn shading_section(&mut self, ui: &mut Ui, _area: Rect) {
        self.shading_prune();
        self.shading_header(ui);
        let spec = shading_spec();
        let rows = self.shading_tree_rows();
        let order: Vec<NodeId> = rows
            .iter()
            .filter_map(|r| match r.key {
                ShaderKey::Layer(id) => Some(id),
                _ => None,
            })
            .collect();
        let mut state = std::mem::take(&mut self.shading.tree);
        let mut editors = std::mem::take(&mut self.text_editors);
        let events = tree_list::show(
            ui,
            &spec,
            &mut state,
            &rows,
            &mut ShadingHooks { app: self },
            &mut editors,
        );
        self.text_editors = editors;
        self.shading.row_rects.clear();
        for (key, _) in &state.rects {
            if let ShaderKey::Layer(id) = *key {
                let cell = |c: usize| {
                    state
                        .cells
                        .get(&(*key, c))
                        .copied()
                        .unwrap_or(Rect::NOTHING)
                };
                self.shading
                    .row_rects
                    .insert(id, [cell(0), cell(1), cell(2), cell(3), cell(4)]);
            }
        }
        // Item 7: the group row a dragged material would fill, outlined
        // in the drop indicator colour (UIC-23).
        if self.preset_browser.drag.is_some()
            && let Some(group) = self.preset_browser.tree_drop
            && let Some((_, rect)) = state
                .rects
                .iter()
                .find(|(k, _)| *k == ShaderKey::Layer(group))
        {
            ui.painter().rect_stroke(
                rect.shrink(1.),
                2.,
                egui::Stroke::new(2_f32, theme::workspace::drop_indicator()),
                egui::StrokeKind::Inside,
            );
        }
        self.shading.tree = state;
        for event in events {
            self.shading_event(ui, event, &order);
        }
    }

    /// Forget selected or collapsed rows that undo or delete took away.
    fn shading_prune(&mut self) {
        let tree = &self.editor.scene.shader_tree;
        self.shading.selected.retain(|&id| tree.get(id).is_some());
        self.shading.collapsed.retain(|&id| tree.get(id).is_some());
        if self.shading.anchor.is_some_and(|id| tree.get(id).is_none()) {
            self.shading.anchor = None;
        }
        let scene = &self.editor.scene;
        let gone = |id: u64| {
            !scene
                .items
                .iter()
                .any(|i| i.id == id && i.environment.is_some())
        };
        if self.shading.environment.is_some_and(|r| gone(r.item())) {
            self.shading.environment = None;
        }
        self.shading
            .tree
            .retain(|k| !matches!(k, ShaderKey::Layer(id) | ShaderKey::Locator(id) if tree.get(id).is_none()));
    }

    /// The two tool rows (UIC-152): View ▾ and Assign Material; Filter ▾,
    /// Add Layer ▾ and the funnel.
    fn shading_header(&mut self, ui: &mut Ui) {
        let mut state = std::mem::take(&mut self.shading.tree);
        let mut assign = false;
        tree_list::tool_row(ui, &mut state, false, |ui, side| {
            if side == tree_list::Side::Left {
                tree_list::tool_dropdown(
                    ui,
                    "shading-view",
                    "View",
                    &mut self.shading.view,
                    &[
                        (ShadingView::ShaderTree, "Shader Tree"),
                        (ShadingView::Materials, "Materials"),
                    ],
                    96.,
                )
                .on_hover_text("Materials is provisional: it shows the whole tree for now");
                // Assign Material opens the Set Material dialog: it names a
                // material for the selected polygons and adds that tag's
                // group to the tree. UIC-03.
                assign = ui
                    .button("Assign Material…")
                    .on_hover_text("Set Material on the selected polygons")
                    .clicked();
            }
        });
        tree_list::tool_row(ui, &mut state, true, |ui, side| {
            if side == tree_list::Side::Left {
                let filters: Vec<(ShadingFilter, &str)> =
                    ShadingFilter::ALL.iter().map(|&f| (f, f.label())).collect();
                tree_list::tool_dropdown(
                    ui,
                    "shading-filter",
                    "Filter",
                    &mut self.shading.filter,
                    &filters,
                    96.,
                );
                ui.menu_button("Add Layer ⏷", |ui| {
                    self.menu_owned_input = true;
                    self.add_layer_menu(ui);
                });
            }
        });
        self.shading.tree = state;
        if assign {
            self.open_set_material();
        }
    }

    /// Every row, top to bottom.
    pub(super) fn shading_tree_rows(&self) -> Vec<TreeRow<ShaderKey>> {
        let s = &self.shading;
        let tree = &self.editor.scene.shader_tree;
        let mut rows = Vec::new();
        let section = |name: &'static str, open: Option<bool>| {
            let mut row = TreeRow::new(ShaderKey::Section(name), 0, name, COLUMNS);
            row.disclosure = open;
            row.fixed = true;
            row
        };
        // Render: the trunk, the root row (bold).
        let mut render = section("Render", Some(!s.render_collapsed));
        render.style.bold = true;
        render.accepts_children = true;
        if self.render_item_selected() {
            render.selected = Selected::Primary;
        }
        rows.push(render);
        if !s.render_collapsed {
            let filter = s.filter;
            let list: Vec<_> = tree
                .rows(&|id| s.collapsed.contains(&id))
                .into_iter()
                .filter(|(n, _)| filter.shows(n) && !tree.in_library(n.id))
                .collect();
            for (i, (node, depth)) in list.iter().enumerate() {
                rows.extend(self.shading_layer_row(node, *depth));
                // NE-11: after a Material Group's last row, a "Nodes (N)"
                // row for its free nodes, outside the layer order.
                let mut closing: Vec<(NodeId, usize)> = list[..=i]
                    .iter()
                    .filter(|(g, _)| {
                        matches!(g.kind, NodeKind::Group(_))
                            && tree.is_within(node.id, g.id)
                            && list
                                .get(i + 1)
                                .is_none_or(|(n, _)| !tree.is_within(n.id, g.id))
                    })
                    .map(|(g, d)| (g.id, *d))
                    .collect();
                closing.sort_by_key(|(_, d)| std::cmp::Reverse(*d));
                for (group, d) in closing {
                    let count = tree.free_node_count(group);
                    if count == 0 || s.collapsed.contains(&group) {
                        continue;
                    }
                    let mut row = TreeRow::new(
                        ShaderKey::Nodes(group),
                        d + 2,
                        format!("Nodes ({count})"),
                        COLUMNS,
                    );
                    row.icon = RowIcon::Icon(crate::icons::Icon::TreeNodeMaterial);
                    row.fixed = true;
                    row.style.italic = true;
                    row.tip = Some(
                        "This material's free nodes; double-click to open its node view".into(),
                    );
                    rows.push(row);
                }
            }
        }
        // The Library (layer menu §4): its items and their contents.
        let has_library = tree.library().next().is_some();
        let library_open = s.open_sections.contains("Library");
        let mut library = section("Library", has_library.then_some(library_open));
        library.accepts_children = true;
        library.tip = Some(
            "Material definitions: drag a Material Group here to keep it; drag an item \
             into the tree to drop a reference to it"
                .into(),
        );
        rows.push(library);
        if has_library && library_open {
            for (node, depth) in tree
                .rows(&|id| s.collapsed.contains(&id))
                .into_iter()
                .filter(|(n, _)| tree.in_library(n.id))
            {
                rows.extend(self.shading_layer_row(node, depth));
            }
        }
        // Nodes (NM-04): the scene's node graphs, shared by Node Materials.
        let graphs = &tree.node_graphs.graphs;
        let nodes_open = s.open_sections.contains("Nodes");
        let mut nodes = section("Nodes", (!graphs.is_empty()).then_some(nodes_open));
        nodes.tip =
            Some("Node graphs that Node Material layers use; double-click one to view it".into());
        rows.push(nodes);
        if nodes_open {
            for g in graphs {
                let mut row = TreeRow::new(ShaderKey::Graph(g.id), 1, &g.material.name, COLUMNS);
                row.icon = RowIcon::Icon(crate::icons::Icon::TreeNodeMaterial);
                row.fixed = true;
                if s.graph == Some(g.id) && s.selected.is_empty() {
                    row.selected = Selected::Primary;
                }
                if !g.unsupported().is_empty() {
                    row.tip = Some("Uses nodes this build can't render".into());
                }
                rows.push(row);
            }
        }
        self.shading_other_sections(&mut rows);
        rows
    }

    /// One layer's rows: the layer, and its Texture Locator attribute row
    /// when that is shown.
    fn shading_layer_row(&self, node: &Node, depth: usize) -> Vec<TreeRow<ShaderKey>> {
        let s = &self.shading;
        let tree = &self.editor.scene.shader_tree;
        let mut rows = Vec::new();
        {
            let mut row = TreeRow::new(ShaderKey::Layer(node.id), depth + 1, &node.name, COLUMNS);
            row.disclosure = tree
                .has_children(node.id)
                .then(|| !s.collapsed.contains(&node.id));
            row.icon = RowIcon::Icon(type_icon(node));
            let has_locator = matches!(&node.kind, NodeKind::Texture(t) if t.locator.is_some());
            // UIC-36: a texture's Texture Locator is an attribute row.
            row.attributes = has_locator.then(|| s.locator_open.contains(&node.id));
            row.style.dimmed = !node.layer.enable;
            // UIC-141: an instance's name is italic.
            row.style.italic = node.extras.instance.is_some();
            // UIC-142: the layer's editor colour.
            row.tint = node.layer.editor_color.map(theme::editor_color);
            row.selected = match s.selected.iter().position(|&x| x == node.id) {
                Some(0) => Selected::Primary,
                Some(_) => Selected::Yes,
                None => Selected::No,
            };
            row.cells[0] = Cell::Toggle {
                dimmed: false,
                on: node.layer.enable,
                tip: if node.layer.enable {
                    "Disable layer · drag down to change several · Alt+click: only this one"
                } else {
                    "Enable layer · drag down to change several · Alt+click: only this one"
                }
                .into(),
            };
            // A reference has no settings of its own (layer menu §4).
            if node.extras.reference.is_none() {
                row.cells[2] = Cell::Custom;
                row.cells[3] = Cell::Custom;
                row.cells[4] = Cell::Custom;
            }
            row.accepts_children = node.is_container();
            row.draggable = true;
            row.renamable = true;
            if matches!(
                &node.kind,
                NodeKind::Texture(t)
                    if matches!(t.source, operandi_core::shader_tree::TextureSource::Procedural(_))
            ) {
                row.tip = Some(
                    "The viewport shows this pattern at the mesh's points (coarse on \
                         low-poly meshes); renders show it fully."
                        .into(),
                );
            }
            if let Some(target) = node.extras.reference {
                let name = tree.get(target).map_or("(missing)", |n| n.name.as_str());
                row.tip = Some(format!("Reference to the Library item {name}"));
            }
            // A Library item keeps its mask; the tooltip says which
            // (provisional, SLM-Q10). A locked one shows the lock.
            // EP v0.2: a material group an environment preset brought.
            if let Some(name) = &node.extras.from_preset {
                row.tip = Some(format!("From preset: {name}"));
            }
            if node.extras.library {
                row.tip = Some(format!("Library item · mask {}", tree.effect_label(node)));
                if node.extras.locked {
                    row.badges.push(tree_list::Badge::Locked);
                }
            }
            #[cfg(feature = "painting")]
            if self.paint_target() == Some(node.id) {
                row.badges.push(tree_list::Badge::PaintTarget);
                row.tip
                    .get_or_insert_with(|| "Paint target: painting goes into this image".into());
            }
            rows.push(row);
            if s.locator_open.contains(&node.id)
                && let NodeKind::Texture(t) = &node.kind
                && let Some(locator) = t.locator
            {
                let name = self
                    .editor
                    .scene
                    .item(locator)
                    .map_or_else(|| "(missing locator)".to_owned(), |i| i.name.clone());
                let mut row = TreeRow::new(ShaderKey::Locator(node.id), depth + 2, name, COLUMNS);
                row.icon = RowIcon::Letter("L");
                row.style.empty = true;
                row.fixed = true;
                row.tip = Some("Texture Locator: click to edit its settings".into());
                rows.push(row);
            }
        }
        rows
    }

    /// Lights, Environments, Bake Items and FX.
    fn shading_other_sections(&self, rows: &mut Vec<TreeRow<ShaderKey>>) {
        let s = &self.shading;
        let section = |name: &'static str, open: Option<bool>| {
            let mut row = TreeRow::new(ShaderKey::Section(name), 0, name, COLUMNS);
            row.disclosure = open;
            row.fixed = true;
            row
        };
        let lights: Vec<String> = self
            .editor
            .scene
            .items
            .iter()
            .filter(|i| i.light.is_some())
            .map(|i| i.name.clone())
            .collect();
        let open = s.open_sections.contains("Lights");
        rows.push(section("Lights", (!lights.is_empty()).then_some(open)));
        if open {
            for (i, child) in lights.into_iter().enumerate() {
                let mut row = TreeRow::new(ShaderKey::Light(i), 1, child, COLUMNS);
                row.fixed = true;
                rows.push(row);
            }
        }
        self.environment_section_rows(rows, section);
        for name in ["Bake Items", "FX"] {
            rows.push(section(name, None));
        }
    }

    fn shading_event(&mut self, ui: &Ui, event: TreeEvent<ShaderKey>, order: &[NodeId]) {
        // Clicking any other row leaves the Render item (RI-02).
        if matches!(event, TreeEvent::Click { key, .. } if key != ShaderKey::Section("Render")) {
            self.shading.render_selected = false;
        }
        match event {
            TreeEvent::Click {
                key: ShaderKey::Section("Render"),
                ..
            } => {
                self.select_render_item();
                self.focus_scene_list(ui, crate::panel_registry::PanelKind::ShaderTree);
            }
            TreeEvent::Click {
                key: ShaderKey::Layer(id),
                how,
            } => {
                let s = &mut self.shading;
                s.environment = None;
                s.graph = None;
                tree_list::click_select(&mut s.selected, &mut s.anchor, order, id, how);
                self.focus_scene_list(ui, crate::panel_registry::PanelKind::ShaderTree);
            }
            TreeEvent::Click {
                key: ShaderKey::Locator(layer),
                ..
            } => {
                // Selects the layer and shows its Texture Locator sub-tab.
                self.shading.selected = vec![layer];
                self.shading.anchor = Some(layer);
                self.shading.layer_tab = LayerTab::TextureLocator;
            }
            TreeEvent::Click {
                key: ShaderKey::Environment(id),
                how,
            } => {
                self.click_environment_row(id, how);
                self.focus_scene_list(ui, crate::panel_registry::PanelKind::ShaderTree);
            }
            TreeEvent::Click {
                key: ShaderKey::EnvironmentMaterial(id),
                ..
            } => self.select_environment_row(crate::environment_ui::EnvironmentRow::Material(id)),
            TreeEvent::Click {
                key: ShaderKey::EnvironmentLayer(item, layer),
                ..
            } => self
                .select_environment_row(crate::environment_ui::EnvironmentRow::Layer(item, layer)),
            TreeEvent::Click {
                key: ShaderKey::Graph(id),
                ..
            } => {
                let s = &mut self.shading;
                s.environment = None;
                s.selected.clear();
                s.anchor = None;
                s.graph = Some(id);
            }
            TreeEvent::Click { .. } => {}
            // NM-15: double-click opens the graph view.
            TreeEvent::DoubleClick(ShaderKey::Graph(id)) => self.open_graph_window(id, None),
            // NE-11: a group's Nodes row opens its node view.
            TreeEvent::DoubleClick(ShaderKey::Nodes(group)) => self.open_group_nodes(group),
            TreeEvent::DoubleClick(ShaderKey::Layer(id))
                if matches!(
                    self.editor.scene.shader_tree.get(id).map(|n| &n.kind),
                    Some(NodeKind::NodeMaterial(m)) if m.graph.is_some()
                ) =>
            {
                if let Some(NodeKind::NodeMaterial(m)) =
                    self.editor.scene.shader_tree.get(id).map(|n| &n.kind)
                    && let Some(g) = m.graph
                {
                    self.open_graph_window(g, Some(id));
                }
            }
            // Material editing item 6: a layer with an image shows it in
            // the Image Viewer. Image layers are leaf rows, so this never
            // meets UIC-38; it isn't a rename either (UIC-39).
            TreeEvent::DoubleClick(ShaderKey::Layer(id))
                if !self.editor.scene.shader_tree.has_children(id)
                    && self.layer_image_path(id).is_some() =>
            {
                if let Some(path) = self.layer_image_path(id) {
                    self.show_image_in_viewer(&path);
                }
            }
            // UIC-38: a parent's double-click selects its children too.
            TreeEvent::DoubleClick(ShaderKey::Layer(id)) => {
                let tree = &self.editor.scene.shader_tree;
                if tree.has_children(id) {
                    let mut ids = vec![id];
                    ids.extend(
                        order
                            .iter()
                            .copied()
                            .filter(|&n| n != id && tree.is_within(n, id)),
                    );
                    self.shading.selected = ids;
                    self.shading.anchor = Some(id);
                }
            }
            TreeEvent::DoubleClick(_) => {}
            TreeEvent::Disclosure { key, subtree } => match key {
                ShaderKey::Section("Render") => {
                    self.shading.render_collapsed = !self.shading.render_collapsed;
                }
                ShaderKey::Section(name) => {
                    if !self.shading.open_sections.remove(name) {
                        self.shading.open_sections.insert(name);
                    }
                }
                ShaderKey::Environment(id) => {
                    if !self.shading.environments_closed.remove(&id) {
                        self.shading.environments_closed.insert(id);
                    }
                }
                ShaderKey::Layer(id) => {
                    let open = !self.shading.collapsed.contains(&id);
                    let tree = &self.editor.scene.shader_tree;
                    // Shift: the whole branch follows (UIC-37).
                    let ids: Vec<NodeId> = if subtree {
                        tree.nodes
                            .iter()
                            .filter(|n| tree.is_within(n.id, id) && tree.has_children(n.id))
                            .map(|n| n.id)
                            .collect()
                    } else {
                        vec![id]
                    };
                    for id in ids {
                        if open {
                            self.shading.collapsed.insert(id);
                        } else {
                            self.shading.collapsed.remove(&id);
                        }
                    }
                }
                ShaderKey::Locator(_)
                | ShaderKey::Light(_)
                | ShaderKey::Graph(_)
                | ShaderKey::Nodes(_)
                | ShaderKey::EnvironmentMaterial(_)
                | ShaderKey::EnvironmentLayer(..) => {}
            },
            TreeEvent::Attributes(ShaderKey::Layer(id)) => {
                if !self.shading.locator_open.remove(&id) {
                    self.shading.locator_open.insert(id);
                }
            }
            TreeEvent::Attributes(_) => {}
            TreeEvent::Rename {
                key: ShaderKey::Layer(id),
                name,
            } => {
                if let Some(node) = self.editor.scene.shader_tree.get(id) {
                    let mut edited = node.clone();
                    edited.name = name;
                    self.replace_layer(edited, false);
                }
            }
            TreeEvent::Rename {
                key: ShaderKey::Environment(id),
                name,
            } => {
                // ENV §6: renamed like any item.
                if self.run_ok(Command::SelectItem(Some(id))) {
                    self.run(Command::Rename(name));
                }
            }
            TreeEvent::Rename {
                key: ShaderKey::EnvironmentMaterial(id),
                name,
            } => self.run(Command::EditEnvironment {
                item: id,
                edit: operandi_core::environment::EnvironmentEdit::MaterialName(name),
            }),
            TreeEvent::Rename {
                key: ShaderKey::EnvironmentLayer(item, layer),
                name,
            } => self.rename_environment_layer(item, layer, name),
            TreeEvent::Rename { .. } => {}
            TreeEvent::Toggle {
                key: ShaderKey::Layer(id),
                solo,
                ..
            } => {
                if solo {
                    self.shading_solo(id);
                } else if let Some(node) = self.editor.scene.shader_tree.get(id) {
                    let enable = !node.layer.enable;
                    self.run(Command::ShaderTree(ShaderTreeEdit::SetEnable {
                        ids: vec![id],
                        enable,
                    }));
                }
            }
            TreeEvent::Toggle {
                key: ShaderKey::EnvironmentMaterial(id),
                ..
            } => {
                if let Some(env) = self.environment(id) {
                    let enable = !env.material.layer.enable;
                    self.run(Command::EditEnvironment {
                        item: id,
                        edit: operandi_core::environment::EnvironmentEdit::Enable(enable),
                    });
                }
            }
            TreeEvent::Toggle {
                key: ShaderKey::EnvironmentLayer(item, layer),
                ..
            } => self.toggle_environment_layer(item, layer),
            // ENVIRONMENT-PLACEMENT §3.2: the Environment row's eye is the
            // item's visibility (item.visible).
            TreeEvent::Toggle {
                key: ShaderKey::Environment(id),
                ..
            } => self.run(Command::Visibility(id)),
            TreeEvent::Toggle { .. } => {}
            TreeEvent::Sweep { keys, on, .. } => {
                let ids: Vec<NodeId> = keys
                    .into_iter()
                    .filter_map(|k| match k {
                        ShaderKey::Layer(id) => Some(id),
                        _ => None,
                    })
                    .collect();
                if !ids.is_empty() {
                    self.run(Command::ShaderTree(ShaderTreeEdit::SetEnable {
                        ids,
                        enable: on,
                    }));
                }
            }
            TreeEvent::Drop { keys, at, .. } => {
                let ids: Vec<NodeId> = keys
                    .into_iter()
                    .filter_map(|k| match k {
                        ShaderKey::Layer(id) => Some(id),
                        _ => None,
                    })
                    .collect();
                let tree = &self.editor.scene.shader_tree;
                // Onto the Library heading: the groups become Library items.
                if matches!(at, DropAt::On(ShaderKey::Section("Library"))) {
                    if !ids.is_empty() {
                        self.run(Command::ShaderTree(ShaderTreeEdit::ToLibrary(ids)));
                    }
                    return;
                }
                // A Library item dragged into the tree drops a reference.
                if let Some(&library) = ids
                    .iter()
                    .find(|&&id| tree.get(id).is_some_and(|n| n.extras.library))
                {
                    if let Some(drop) = self.shading_drop(at) {
                        let id = self.editor.scene.shader_tree.next_id;
                        if self.run_ok(Command::ShaderTree(ShaderTreeEdit::AddReference {
                            library,
                            drop,
                        })) {
                            self.shading.selected = vec![id];
                            self.shading.anchor = Some(id);
                        }
                    }
                    return;
                }
                if let Some(drop) = self.shading_drop(at)
                    && !ids.is_empty()
                {
                    self.run(Command::ShaderTree(ShaderTreeEdit::Move { ids, drop }));
                }
            }
        }
    }

    /// Where a drop lands in the tree (UIC-143): onto a Group/Folder or
    /// Render as its first child; between layers at that layer's level;
    /// below the last row at the end of Render.
    fn shading_drop(&self, at: DropAt<ShaderKey>) -> Option<TreeDrop> {
        let tree = &self.editor.scene.shader_tree;
        let place = |id: NodeId, after: bool| {
            let node = tree.get(id)?;
            let index = tree.children(node.parent).position(|n| n.id == id)?;
            Some(TreeDrop {
                parent: node.parent,
                index: index + usize::from(after),
            })
        };
        match at {
            DropAt::On(ShaderKey::Layer(id)) => Some(TreeDrop {
                parent: Some(id),
                index: 0,
            }),
            DropAt::On(ShaderKey::Section("Render")) => Some(TreeDrop {
                parent: None,
                index: 0,
            }),
            // Before or after a Library row: inside the Library item.
            DropAt::Before(ShaderKey::Section("Library")) => Some(TreeDrop {
                parent: None,
                index: tree
                    .children(None)
                    .take_while(|n| !n.extras.library)
                    .count(),
            }),
            DropAt::Before(ShaderKey::Layer(id)) => place(id, false),
            DropAt::After(ShaderKey::Layer(id)) => place(id, true),
            DropAt::End => Some(TreeDrop {
                parent: None,
                index: tree.children(None).count(),
            }),
            _ => None,
        }
    }

    /// UIC-138 Alt+click: enable only this layer; Alt+click it again turns
    /// the others back on. One undo step each way.
    fn shading_solo(&mut self, id: NodeId) {
        let tree = &self.editor.scene.shader_tree;
        if let Some((solo, others)) = self.shading.solo.take()
            && solo == id
        {
            let ids: Vec<NodeId> = others
                .into_iter()
                .filter(|&n| tree.get(n).is_some())
                .collect();
            if !ids.is_empty() {
                self.run(Command::ShaderTree(ShaderTreeEdit::SetEnable {
                    ids,
                    enable: true,
                }));
            }
            return;
        }
        // Its own ancestors stay on, or it would show nothing.
        let others: Vec<NodeId> = tree
            .nodes
            .iter()
            .filter(|n| n.id != id && n.layer.enable && !tree.is_within(id, n.id))
            .map(|n| n.id)
            .collect();
        if tree.get(id).is_some_and(|n| !n.layer.enable) {
            self.run(Command::ShaderTree(ShaderTreeEdit::SetEnable {
                ids: vec![id],
                enable: true,
            }));
        }
        if !others.is_empty() {
            self.run(Command::ShaderTree(ShaderTreeEdit::SetEnable {
                ids: others.clone(),
                enable: false,
            }));
        }
        self.shading.solo = Some((id, others));
    }

    /// A layer's right-click menu (shader layer menu §1). Entries that
    /// don't apply to the clicked layer are left out, not greyed (UIC-07).
    fn layer_menu(&mut self, ui: &mut Ui, id: NodeId, rename: &mut dyn FnMut()) {
        let Some(node) = self.editor.scene.shader_tree.get(id).cloned() else {
            return;
        };
        self.menu_owned_input = true;
        if !self.shading.selected.contains(&id) {
            self.shading.selected = vec![id];
            self.shading.anchor = Some(id);
        }
        if ui.button("Rename…").clicked() {
            rename();
            ui.close();
        }
        // Note 10 (layer menu §7): the selected layers' Editor Color, one undo step.
        if let Some(color) = crate::editor_color_ui::submenu(ui, node.layer.editor_color) {
            self.run(Command::ShaderTree(ShaderTreeEdit::SetEditorColor {
                ids: self.shading.selected.clone(),
                color,
            }));
        }
        if ui.button("Properties").clicked() {
            self.shading.selected = vec![id];
            self.shading.anchor = Some(id);
            self.show_properties_panel();
            ui.close();
        }
        // NM-15: the layer's graph, read-only, in the Node View panel.
        if let NodeKind::NodeMaterial(m) = &node.kind
            && let Some(graph) = m.graph
            && ui.button("Show Graph").clicked()
        {
            self.open_graph_window(graph, Some(id));
            ui.close();
        }
        if self.editor.scene.shader_tree.polygon_group(id).is_some()
            && ui
                .button("Select Polygons")
                .on_hover_text("Select the polygons this Material Group's mask matches")
                .clicked()
        {
            self.run(Command::ShaderTree(ShaderTreeEdit::SelectPolygons(id)));
            ui.close();
        }
        ui.separator();
        if ui.button("Duplicate").clicked() {
            self.shading_duplicate();
            ui.close();
        }
        if operandi_core::shader_instances::can_instance(&node)
            && ui
                .button("Create Instance")
                .on_hover_text("A linked copy: it follows this layer except where you change it")
                .clicked()
        {
            let ids = self.shading.selected.clone();
            let first = self.editor.scene.shader_tree.next_id;
            if self.run_ok(Command::ShaderTree(ShaderTreeEdit::CreateInstance(ids))) {
                self.select_new_layers(first);
            }
            ui.close();
        }
        if node.extras.reference.is_some() {
            ui.menu_button("References", |ui| {
                if ui
                    .button("De-Reference")
                    .on_hover_text("Turn the reference into a real copy of the Library layers")
                    .clicked()
                {
                    let ids: Vec<NodeId> = self
                        .shading
                        .selected
                        .iter()
                        .copied()
                        .filter(|&s| self.editor.scene.shader_tree.reference_target(s).is_some())
                        .collect();
                    let first = self.editor.scene.shader_tree.next_id;
                    if self.run_ok(Command::ShaderTree(ShaderTreeEdit::DeReference(ids))) {
                        self.select_new_layers(first);
                    }
                    ui.close();
                }
            });
        }
        if node.extras.library {
            let locked = node.extras.locked;
            let label = if locked {
                "Unlock Preset"
            } else {
                "Lock Preset"
            };
            if ui
                .button(label)
                .on_hover_text(if locked {
                    "Allow edits to this Library item again"
                } else {
                    "Keep this Library item's settings from being edited"
                })
                .clicked()
            {
                self.run(Command::ShaderTree(ShaderTreeEdit::SetLocked {
                    node: id,
                    locked: !locked,
                }));
                ui.close();
            }
        }
        let instances = self.editor.scene.shader_tree.instances_of(id);
        if !instances.is_empty() && ui.button("Select Instances").clicked() {
            self.shading.anchor = instances.first().copied();
            self.shading.selected = instances;
            ui.close();
        }
        if let Some(source) = self.editor.scene.shader_tree.instance_source(id)
            && ui.button("Select Source of Instance").clicked()
        {
            self.shading.selected = vec![source];
            self.shading.anchor = Some(source);
            ui.close();
        }
        ui.separator();
        if ui.button("Copy").clicked() {
            self.shading_copy();
            ui.close();
        }
        if self.shading.clipboard.is_some() {
            if ui.button("Paste").clicked() {
                self.shading_paste();
                ui.close();
            }
            if ui
                .button("Paste Values")
                .on_hover_text(
                    "Copy the copied layer's settings onto the selected layers of its type",
                )
                .clicked()
            {
                self.shading_paste_values();
                ui.close();
            }
        }
        if ui.button("Delete").clicked() {
            self.shading_delete_selected();
            ui.close();
        }
        // PB-41 / PB-51: materials and material groups save as presets; a
        // material group takes one in. UIC-03: both open a file panel.
        // UIC-07: other rows get no empty submenu.
        let group = matches!(node.kind, NodeKind::Group(_));
        if group || matches!(node.kind, NodeKind::Material(_)) {
            ui.separator();
            ui.menu_button("Presets", |ui| {
                if ui.button("Save…").clicked() {
                    self.request_save_preset(operandi_core::preset::PresetSource::Material(id));
                    ui.close();
                }
                if group && ui.button("Load…").clicked() {
                    self.request_load_preset(Some(id));
                    ui.close();
                }
            });
        }
        // Node editor slice 1: the group's MaterialX graph, read-only, in
        // the Node View panel (UIC-244: the graph renders use; UIC-157).
        if group
            && ui
                .button("Show as Nodes")
                .on_hover_text("Show this material's nodes in the Node View panel")
                .clicked()
        {
            self.execute(&format!("operandi.nodeMaterial.showGraph layer:{id}"));
            ui.close();
        }
        // Change Type ▸ (§6): texture layers only, Add Layer's texture
        // types grouped as there; the layer's own type is left out.
        if let NodeKind::Texture(t) = &node.kind
            && node.extras.instance.is_none()
        {
            if !(group || matches!(node.kind, NodeKind::Material(_))) {
                ui.separator();
            }
            let current = t.source.label();
            let mut chosen = None;
            let mut pick = |ui: &mut Ui, layer: NewLayer| {
                if layer.label() != current && ui.button(layer.label()).clicked() {
                    chosen = Some(layer);
                    ui.close();
                }
            };
            ui.menu_button("Change Type", |ui| {
                pick(ui, NewLayer::Constant);
                pick(ui, NewLayer::ImageMap);
                ui.menu_button("Processing", |ui| {
                    pick(ui, NewLayer::Gradient);
                    pick(ui, NewLayer::VertexMap);
                    pick(ui, NewLayer::Variation);
                    pick(ui, NewLayer::Process);
                    pick(ui, NewLayer::Occlusion);
                });
                ui.menu_button("Textures", |ui| {
                    use operandi_core::procedural::ProceduralKind;
                    for kind in ProceduralKind::ALL {
                        pick(ui, NewLayer::Procedural(*kind));
                    }
                });
            });
            if let Some(to) = chosen {
                self.run(Command::ShaderTree(ShaderTreeEdit::ChangeType {
                    node: id,
                    to,
                }));
            }
        }
        // Image Map layers (§7).
        if let NodeKind::Texture(t) = &node.kind
            && let operandi_core::shader_tree::TextureSource::ImageMap { image, .. } = &t.source
        {
            let path = image.and_then(|i| {
                self.editor
                    .scene
                    .images
                    .iter()
                    .find(|x| x.id == i)
                    .map(|x| x.path.clone())
            });
            if let Some(path) = &path
                && ui
                    .button("Reload Image")
                    .on_hover_text("Read the picture file again")
                    .clicked()
            {
                // Through the command (UIC-160), so scripts do the same.
                self.execute(&format!("clip.reload \"{path}\""));
                ui.close();
            }
            if ui
                .button("Replace Image…")
                .on_hover_text("Pick another picture; the layer keeps its settings")
                .clicked()
            {
                let folder = path
                    .as_deref()
                    .and_then(|p| std::path::Path::new(p).parent().map(|p| p.to_path_buf()))
                    .unwrap_or_default();
                if !self.choose(
                    crate::file_chooser::Choose::Image { folder },
                    crate::file_chooser::Purpose::ShaderImage(id),
                ) {
                    self.status = "No file panel is available to choose a picture".into();
                }
                ui.close();
            }
            if let Some(path) = &path
                && ui
                    .button("Open Image Folder")
                    .on_hover_text(crate::image_viewer_ui::reveal_label())
                    .clicked()
            {
                if let Err(e) = crate::bug_report::reveal(std::path::Path::new(path)) {
                    self.status = format!("Couldn't show the picture: {e}");
                }
                ui.close();
            }
        }
    }

    /// Bring the Properties panel to the front (adding it to the lower
    /// group when no group shows it).
    pub(super) fn show_properties_panel(&mut self) {
        use panel_registry::PanelKind;
        if let Some(pane) = self.registry_dock_pane(PanelKind::Properties) {
            self.views.reveal_docked_tab(pane);
            return;
        }
        for (group, pane) in [
            (&mut self.lower_frame, operandi_layout::Pane::Properties),
            (&mut self.upper_frame, operandi_layout::Pane::Items),
        ] {
            if let Some(i) = group.tabs.iter().position(|&k| k == PanelKind::Properties) {
                group.show_page(i);
                self.views.reveal_docked_tab(pane);
                return;
            }
        }
        if self.layouts.mixed_dock && self.lower_frame.tabs.is_empty() {
            self.upper_frame.add(PanelKind::Properties);
            self.views.reveal_docked_tab(operandi_layout::Pane::Items);
        } else {
            self.lower_frame.add(PanelKind::Properties);
            self.views
                .reveal_docked_tab(operandi_layout::Pane::Properties);
        }
    }

    /// Select the layers an edit just added (ids from `first` on that sit
    /// directly in a parent that isn't new).
    pub(super) fn select_new_layers(&mut self, first: NodeId) {
        let tree = &self.editor.scene.shader_tree;
        let new: Vec<NodeId> = tree.top_level_since(first).into_iter().collect();
        if !new.is_empty() {
            self.shading.anchor = new.first().copied();
            self.shading.selected = new;
        }
    }

    pub(super) fn shading_duplicate(&mut self) {
        let ids = self.shading.selected.clone();
        if ids.is_empty() {
            return;
        }
        let first = self.editor.scene.shader_tree.next_id;
        if self.run_ok(Command::ShaderTree(ShaderTreeEdit::Duplicate(ids))) {
            self.select_new_layers(first);
        }
    }

    pub(super) fn shading_copy(&mut self) {
        let ids = self.shading.selected.clone();
        if ids.is_empty() {
            return;
        }
        let clip = self.editor.scene.copy_shader_layers(&ids);
        let roots = clip.roots().count();
        self.status = format!(
            "Copied {roots} shader layer{}",
            if roots == 1 { "" } else { "s" }
        );
        self.shading.clipboard = Some(Box::new(clip));
    }

    pub(super) fn shading_paste(&mut self) {
        let Some(clip) = self.shading.clipboard.clone() else {
            return;
        };
        let first = self.editor.scene.shader_tree.next_id;
        let selected = self.shading.selected.first().copied();
        if self.run_ok(Command::ShaderTree(ShaderTreeEdit::Paste {
            clip,
            selected,
        })) {
            self.select_new_layers(first);
        }
    }

    /// Paste Values: the copied layer of the selection's type (the first
    /// copied layer when none matches, which the edit refuses).
    pub(super) fn shading_paste_values(&mut self) {
        let Some(clip) = &self.shading.clipboard else {
            return;
        };
        let targets = self.shading.selected.clone();
        let tree = &self.editor.scene.shader_tree;
        let wanted = targets
            .first()
            .and_then(|&id| tree.get(id))
            .map(|n| n.type_label());
        let from = clip
            .roots()
            .find(|n| Some(n.type_label()) == wanted)
            .or_else(|| clip.roots().next())
            .cloned();
        if let Some(from) = from {
            self.run(Command::ShaderTree(ShaderTreeEdit::PasteValues {
                from: Box::new(from),
                targets,
            }));
        }
    }

    fn add_layer_menu(&mut self, ui: &mut Ui) {
        let mut chosen = None;
        // The Material Values sub-menu's pick (its buttons carry tips).
        let mut chosen_value = None;
        let mut pick = |ui: &mut Ui, layer: NewLayer| {
            if ui.button(layer.label()).clicked() {
                chosen = Some(layer);
                ui.close();
            }
        };
        ui.menu_button("Special", |ui| {
            pick(ui, NewLayer::Folder);
            pick(ui, NewLayer::Group);
            pick(ui, NewLayer::Shader);
            ui.menu_button("Render Output", |ui| {
                for &e in OutputEffect::ALL {
                    if e.material_value().is_none() {
                        pick(ui, NewLayer::RenderOutput(e));
                    }
                }
                // Material-value outputs in their own sub-menu (UIC-10),
                // one plain line each on what they write (UIC-04).
                ui.separator();
                ui.menu_button("Material Values", |ui| {
                    for &e in OutputEffect::MATERIAL_VALUES {
                        let layer = NewLayer::RenderOutput(e);
                        let button = ui.button(layer.label());
                        let button = match e.material_value_tip() {
                            Some(tip) => button.on_hover_text(tip),
                            None => button,
                        };
                        if button.clicked() {
                            chosen_value = Some(layer);
                            ui.close();
                        }
                    }
                });
            });
        });
        pick(ui, NewLayer::Material);
        // NM-10: a finished surface from a node graph.
        pick(ui, NewLayer::NodeMaterial);
        let import = ui
            .button("Import Node Material…")
            .on_hover_text("Node graphs from a node graph file (.json), each as a Node Material")
            .clicked();
        if import {
            ui.close();
        }
        pick(ui, NewLayer::ImageMap);
        ui.menu_button("Processing", |ui| {
            pick(ui, NewLayer::Constant);
            pick(ui, NewLayer::Gradient);
            pick(ui, NewLayer::VertexMap);
            pick(ui, NewLayer::Variation);
            pick(ui, NewLayer::Process);
            pick(ui, NewLayer::Occlusion);
        });
        ui.menu_button("Textures", |ui| {
            use operandi_core::procedural::ProceduralKind;
            for kind in ProceduralKind::ALL {
                pick(ui, NewLayer::Procedural(*kind));
            }
        });
        // ENV §8: a whole new environment item.
        let environment = ui.button("Environment").clicked();
        if environment {
            ui.close();
        }
        if let Some(layer) = chosen.or(chosen_value) {
            // ENV §8: with an environment row selected, texture entries go
            // into that environment.
            if self.shading.environment.is_some() {
                self.add_environment_layer(layer);
            } else {
                self.shading_add(layer);
            }
        }
        if environment {
            self.create_environment();
        }
        if import {
            self.request_import_node_material();
        }
    }

    pub(super) fn shading_add(&mut self, layer: NewLayer) {
        let id = self.editor.scene.shader_tree.next_id;
        let selected = self.shading.selected.last().copied();
        if self.run_ok(Command::ShaderTree(ShaderTreeEdit::Add { layer, selected })) {
            self.shading.selected = vec![id];
            self.shading.anchor = Some(id);
            // Show the new layer inside a collapsed parent.
            let mut at = self.editor.scene.shader_tree.get(id).and_then(|n| n.parent);
            while let Some(p) = at {
                self.shading.collapsed.remove(&p);
                at = self.editor.scene.shader_tree.get(p).and_then(|n| n.parent);
            }
            // #56 (Jason, 2026-11-27): a new Image Map asks for its picture
            // at once; cancelling keeps the empty layer.
            if layer == NewLayer::ImageMap {
                self.choose_shader_image(id);
            }
        }
    }

    /// Delete the selected layers (Delete key with the list focused, or the
    /// row menu). False when nothing is selected.
    pub(super) fn shading_delete_selected(&mut self) -> bool {
        // ENV §6: deleting an environment row deletes the item and its
        // layers. Its Environment Material can't be deleted alone [chosen].
        match self.shading.environment {
            Some(crate::environment_ui::EnvironmentRow::Item(id)) => {
                self.delete_environment(id);
                return true;
            }
            Some(crate::environment_ui::EnvironmentRow::Layer(item, layer)) => {
                self.run(Command::EditEnvironmentLayers {
                    item,
                    edit: operandi_core::environment::EnvironmentLayerEdit::Remove(vec![layer]),
                });
                self.shading.environment = Some(crate::environment_ui::EnvironmentRow::Item(item));
                return true;
            }
            _ => {}
        }
        if self.shading.selected.is_empty() {
            return false;
        }
        let ids = std::mem::take(&mut self.shading.selected);
        // Deleting a source deletes its instances too, a Library item its
        // references: ask first (§3, §4).
        let tree = &self.editor.scene.shader_tree;
        if tree.instances_deleted_with(&ids) > 0 || !tree.references_deleted_with(&ids).is_empty() {
            self.shading.selected = ids.clone();
            self.shading.confirm_delete = Some(ids);
            return true;
        }
        self.run(Command::ShaderTree(ShaderTreeEdit::Remove(ids)));
        true
    }

    /// The delete confirmation (UIC-97): "Also deletes n instance(s)".
    pub(super) fn shading_delete_confirmation(&mut self, ctx: &Context) {
        let Some(ids) = self.shading.confirm_delete.clone() else {
            return;
        };
        let tree = &self.editor.scene.shader_tree;
        let refs = tree.references_deleted_with(&ids).len();
        if refs > 0 {
            // Yes: De-Reference them first (they become copies); No: delete
            // them too; Cancel: nothing (provisional, SLM-Q11).
            let message = format!(
                "{refs} reference{} use{} this Library item. De-Reference {} first, so {} \
                 real cop{}? No deletes {} too.",
                if refs == 1 { "" } else { "s" },
                if refs == 1 { "s" } else { "" },
                if refs == 1 { "it" } else { "them" },
                if refs == 1 {
                    "it stays as a"
                } else {
                    "they stay as"
                },
                if refs == 1 { "y" } else { "ies" },
                if refs == 1 { "it" } else { "them" },
            );
            match self.show_question(ctx, "Delete", &message) {
                form::Outcome::Ok => {
                    self.shading.confirm_delete = None;
                    self.shading.selected.clear();
                    self.run(Command::ShaderTree(ShaderTreeEdit::DeReferenceAndRemove(
                        ids,
                    )));
                }
                form::Outcome::No => {
                    self.shading.confirm_delete = None;
                    self.shading.selected.clear();
                    self.run(Command::ShaderTree(ShaderTreeEdit::Remove(ids)));
                }
                form::Outcome::Open => {}
                _ => self.shading.confirm_delete = None,
            }
            return;
        }
        let n = tree.instances_deleted_with(&ids);
        let layers = ids.len();
        let message = format!(
            "Delete {layers} shader layer{}? Also deletes {n} instance{}.",
            if layers == 1 { "" } else { "s" },
            if n == 1 { "" } else { "s" },
        );
        match self.show_confirm(ctx, "Delete", &message, &[]) {
            form::Outcome::Ok => {
                self.shading.confirm_delete = None;
                self.shading.selected.clear();
                self.run(Command::ShaderTree(ShaderTreeEdit::Remove(ids)));
            }
            form::Outcome::Open => {}
            _ => self.shading.confirm_delete = None,
        }
    }

    fn blend_cell(&mut self, ui: &mut Ui, cell: Rect, node: &Node) {
        let response = ui.interact(cell, Id::new(("shader-blend", node.id)), Sense::click());
        ui.painter().text(
            cell.left_center() + vec2(3., 0.),
            Align2::LEFT_CENTER,
            short_blend(node.layer.blend),
            FontId::proportional(10.),
            theme::text(),
        );
        let hover = match node.layer.blend {
            BlendMode::Normal => "Blend Mode: Normal".to_owned(),
            mode => format!("Blend Mode: {}", mode.label()),
        };
        let response = response.on_hover_text(hover);
        let node_material = matches!(node.kind, NodeKind::NodeMaterial(_));
        Popup::menu(&response).show(|ui| {
            self.menu_owned_input = true;
            for &mode in BlendMode::ALL {
                // NM-12: Normal only for Node Materials in this version.
                let why = crate::unavailable::unless(
                    !node_material || mode == BlendMode::Normal,
                    "Node Materials blend Normal only for now",
                );
                if ui
                    .gated(
                        why,
                        egui::Button::selectable(node.layer.blend == mode, mode.label()),
                    )
                    .clicked()
                {
                    let mut edited = node.clone();
                    edited.layer.blend = mode;
                    self.replace_layer(edited, false);
                    ui.close();
                }
            }
        });
    }

    /// % column: drag left/right to change opacity; one undo step per drag.
    fn opacity_cell(&mut self, ui: &mut Ui, cell: Rect, node: &Node) {
        let response = ui.interact(cell, Id::new(("shader-opacity", node.id)), Sense::drag());
        ui.painter().text(
            cell.right_center() - vec2(4., 0.),
            Align2::RIGHT_CENTER,
            // N14: the column's header says "%", so rows show the number.
            format!("{:.0}", node.layer.opacity * 100.),
            FontId::proportional(11.),
            theme::text(),
        );
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
        }
        if response.dragged() {
            let dx = response.drag_delta().x as f64;
            if dx != 0. {
                let mut edited = node.clone();
                edited.layer.opacity = (node.layer.opacity + dx * 0.005).clamp(0., 1.);
                let continuing = self.shading.opacity_drag == Some(node.id);
                self.shading.opacity_drag = Some(node.id);
                self.replace_layer(edited, continuing);
            }
        }
        if response.drag_stopped() {
            self.shading.opacity_drag = None;
        }
        response.on_hover_text("Opacity · drag to change");
    }

    fn effect_cell(&mut self, ui: &mut Ui, cell: Rect, node: &Node) {
        let label = self.editor.scene.shader_tree.effect_label(node);
        let menu = matches!(
            node.kind,
            NodeKind::Texture(_) | NodeKind::Shader(_) | NodeKind::RenderOutput(_)
        );
        ui.painter().text(
            cell.left_center() + vec2(3., 0.),
            Align2::LEFT_CENTER,
            if menu { format!("{label} ▾") } else { label },
            FontId::proportional(10.),
            theme::text(),
        );
        if !menu {
            return;
        }
        let response = ui.interact(cell, Id::new(("shader-effect", node.id)), Sense::click());
        // Left or right click opens the menu (§4.2).
        let popup = if response.secondary_clicked() {
            Popup::context_menu(&response)
        } else {
            Popup::menu(&response)
        };
        let inputs = self.editor.scene.shader_tree.node_inputs_for(node.id);
        popup.show(|ui| {
            self.menu_owned_input = true;
            if let Some(edited) = effect_menu(ui, node, &inputs) {
                self.replace_layer(edited, false);
                ui.close();
            }
        });
    }

    /// The Edit Gradient window (GR-Q03 v2) for its layer, while that layer
    /// is a Gradient; each change is a layer edit (one undo step, one per
    /// drag), as in the panel.
    pub(super) fn gradient_editor_window(&mut self, ctx: &egui::Context) {
        let Some(id) = self.shading.gradient_window else {
            return;
        };
        let Some(node) = self.editor.scene.shader_tree.get(id).cloned() else {
            self.shading.gradient_window = None;
            return;
        };
        let mut edited = node.clone();
        let NodeKind::Texture(t) = &mut edited.kind else {
            self.shading.gradient_window = None;
            return;
        };
        let color = t.effect == TextureEffect::Rgba
            || operandi_core::shader_eval::Ch::of_effect(t.effect).is_some_and(|c| c.is_color());
        let operandi_core::shader_tree::TextureSource::Gradient(g) = &mut t.source else {
            self.shading.gradient_window = None;
            return;
        };
        let (open, dragging) = crate::gradient_ui::window(
            ctx,
            &node.name,
            g,
            color,
            &mut self.shading.gradient_recent,
        );
        if !open {
            crate::gradient_ui::remember(&mut self.shading.gradient_recent, g);
            self.shading.gradient_window = None;
        }
        if edited != node {
            self.replace_layer(edited, dragging);
        }
    }

    pub(super) fn replace_layer(&mut self, node: Node, continuing: bool) {
        // A one-setting edit runs its command (UIC-164/165): item.name or
        // item.channel layer:<id>. Core builds the Replace from the line;
        // it's used only when it is exactly this node, so behaviour can't
        // drift, and a drag keeps folding (`continuing`).
        if let Some(line) = self.layer_edit_line(&node)
            && let Some(Ok(Some(Command::ShaderTree(ShaderTreeEdit::Replace {
                node: built, ..
            })))) = operandi_core::command_line::command_for_line(&mut self.editor, &line)
            && built == node
        {
            #[cfg(test)]
            crate::ui_inventory::note_command(&line);
            self.run(Command::ShaderTree(ShaderTreeEdit::Replace {
                node: built,
                continuing,
            }));
            return;
        }
        self.run(Command::ShaderTree(ShaderTreeEdit::Replace {
            node,
            continuing,
        }));
    }

    /// The command line for `node`'s edit when it changes one setting.
    pub(super) fn layer_edit_line(&self, node: &Node) -> Option<String> {
        use operandi_core::channels::json_leaves;
        let old = self.editor.scene.shader_tree.get(node.id)?;
        let id = node.id;
        let brace = |t: String| {
            if t.contains(' ') || t.is_empty() {
                format!("{{{t}}}")
            } else {
                t
            }
        };
        let same_but = |a: &Node, b: &Node| {
            (
                a.parent == b.parent && a.extras == b.extras,
                a.kind == b.kind,
                a.layer == b.layer,
            )
        };
        let (rest_same, kind_same, layer_same) = same_but(old, node);
        if !rest_same {
            return None;
        }
        if kind_same && layer_same && old.name != node.name {
            return Some(format!("item.name {} layer:{id}", brace(node.name.clone())));
        }
        if old.name != node.name {
            return None;
        }
        if kind_same && !layer_same {
            let (a, b) = (&old.layer, &node.layer);
            let changed: Vec<(&str, String)> = [
                (a.enable != b.enable).then(|| ("enable", b.enable.to_string())),
                (a.invert != b.invert).then(|| ("invert", b.invert.to_string())),
                (a.blend != b.blend).then(|| ("blend", brace(b.blend.label().to_owned()))),
                (a.opacity != b.opacity).then(|| ("opacity", b.opacity.to_string())),
            ]
            .into_iter()
            .flatten()
            .collect();
            return match changed.as_slice() {
                [(name, value)] => Some(format!("item.channel {name} {value} layer:{id}")),
                _ => None,
            };
        }
        if !layer_same {
            return None;
        }
        if let Some(line) = map_layer_line(old, node) {
            return Some(line);
        }
        let payload = |n: &Node| operandi_core::channels::kind_payload(&n.kind);
        let ((va, pa), (vb, pb)) = (payload(old)?, payload(node)?);
        if va != vb {
            return None;
        }
        let (mut was, mut now) = (Vec::new(), Vec::new());
        json_leaves("", &pa, &mut was);
        json_leaves("", &pb, &mut now);
        // A kind that is one value (a Render Output's effect).
        for (path, _) in was.iter_mut().chain(now.iter_mut()) {
            if path.is_empty() {
                *path = "effect".into();
            }
        }
        if was.len() != now.len() {
            return None;
        }
        let changed: Vec<_> = was.iter().zip(&now).filter(|(a, b)| a != b).collect();
        let [(_, (path, leaf))] = changed.as_slice() else {
            return None;
        };
        let text = match leaf {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Array(parts) => parts
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(","),
            other => other.to_string(),
        };
        Some(format!("item.channel {path} {} layer:{id}", brace(text)))
    }

    /// Properties for the selected shader tree layer(s).
    pub(super) fn shading_properties(&mut self, ui: &mut Ui) {
        match self.shading.environment {
            Some(crate::environment_ui::EnvironmentRow::Item(id)) => {
                return self.environment_item_rows(ui, id);
            }
            Some(crate::environment_ui::EnvironmentRow::Material(id)) => {
                return self.environment_material_rows(ui, id);
            }
            Some(crate::environment_ui::EnvironmentRow::Layer(item, layer)) => {
                return self.environment_layer_rows(ui, item, layer);
            }
            None => {}
        }
        if self.render_item_selected() {
            // UIC-101: a tab Render doesn't have shows the first.
            let tab = crate::render_item_ui::TABS
                .into_iter()
                .find(|t| *t == self.properties_tab)
                .unwrap_or(crate::render_item_ui::FRAME_TAB);
            return self.render_item_rows(ui, tab);
        }
        let tree = &self.editor.scene.shader_tree;
        let nodes: Vec<Node> = self
            .shading
            .selected
            .iter()
            .filter_map(|&id| tree.get(id).cloned())
            .collect();
        let Some(node) = nodes.last().cloned() else {
            if let Some(graph) = self.shading.graph {
                return self.node_graph_properties(ui, graph);
            }
            ui.weak("Select a layer in the shader tree");
            return;
        };
        if nodes.len() > 1 {
            // Gang-editing several layers of one type comes later.
            ui.weak(format!(
                "{} layers selected · showing {}",
                nodes.len(),
                node.name
            ));
        }
        ui.strong(node.type_label());
        // An instance (layer menu §3): its source, and its overridden
        // settings marked in italics with Revert to Source.
        let overridden = operandi_core::shader_instances::overridden_labels(&node);
        if let Some(source) = self.editor.scene.shader_tree.instance_source(node.id) {
            let name = self
                .editor
                .scene
                .shader_tree
                .get(source)
                .map_or("(missing)".to_owned(), |n| n.name.clone());
            ui.weak(format!("Instance of {name}"));
            if !overridden.is_empty()
                && ui
                    .button("Revert All to Source")
                    .on_hover_text("Drop every override: follow the source again")
                    .clicked()
            {
                self.run(Command::ShaderTree(ShaderTreeEdit::RevertToSource {
                    node: node.id,
                    paths: None,
                }));
                return;
            }
        }
        // A reference has no settings of its own (layer menu §4).
        if let Some(target) = node.extras.reference {
            let tree = &self.editor.scene.shader_tree;
            let name = tree
                .get(target)
                .map_or("(missing)".to_owned(), |n| n.name.clone());
            ui.weak(format!("Reference to the Library item {name}"));
            ui.weak("Edit the Library item to change what it shades.");
            if ui
                .button("De-Reference")
                .on_hover_text("Turn the reference into a real copy of the Library layers")
                .clicked()
            {
                let first = tree.next_id;
                if self.run_ok(Command::ShaderTree(ShaderTreeEdit::DeReference(vec![
                    node.id,
                ]))) {
                    self.select_new_layers(first);
                }
            }
            return;
        }
        // A locked Library item's fields are read-only (layer menu §4).
        let locked = self.editor.scene.shader_tree.is_locked(node.id);
        if locked {
            ui.weak("Locked Library item: Unlock Preset to edit it");
        }
        crate::form::rows::mark_overridden(
            overridden
                .iter()
                .filter(|(l, _)| !l.is_empty())
                .map(|(l, _)| (*l).to_owned()),
        );
        ui.gated_ui(
            crate::unavailable::unless(!locked, "Locked Library item: Unlock Preset to edit it"),
            |ui| self.shading_layer_rows(ui, &node, &nodes),
        );
        crate::form::rows::clear_overridden();
        if let Some(label) = crate::form::rows::take_revert()
            && let Some((_, paths)) = overridden.iter().find(|(l, _)| *l == label)
        {
            self.run(Command::ShaderTree(ShaderTreeEdit::RevertToSource {
                node: node.id,
                paths: Some(paths.clone()),
            }));
        }
    }

    /// The Properties rows of layer `node` (`nodes`: every selected layer).
    fn shading_layer_rows(&mut self, ui: &mut Ui, node: &Node, nodes: &[Node]) {
        let node = node.clone();
        // A texture layer's Properties have two sub-tabs.
        if matches!(node.kind, NodeKind::Texture(_)) {
            ui.horizontal(|ui| {
                let tab = &mut self.shading.layer_tab;
                ui.selectable_value(tab, LayerTab::Layer, "Layer");
                ui.selectable_value(tab, LayerTab::TextureLocator, "Texture Locator");
            });
            if self.shading.layer_tab == LayerTab::TextureLocator {
                self.texture_locator_properties(ui, &node);
                return;
            }
        }
        let mut edited = node.clone();
        let mut continuing = false;
        // Colour swatch drags to carry to the other selected layers (A5).
        let mut gangs: Vec<(&'static str, crate::colour_field::GangDrag)> = Vec::new();
        // UIC-55: a preset list was edited; written to disk below.
        let mut preset_saved = false;
        {
            {
                crate::option_row::field_row(ui, "Name", |ui| {
                    if self
                        .shading
                        .name_draft
                        .as_ref()
                        .is_none_or(|(id, _)| *id != node.id)
                    {
                        self.shading.name_draft = Some((node.id, node.name.clone()));
                    }
                    if let Some((_, draft)) = &mut self.shading.name_draft {
                        let response = ui.add(
                            TextEdit::singleline(draft)
                                .desired_width(ui.available_width())
                                .horizontal_align(Align::RIGHT)
                                .background_color(palette::list())
                                .text_color(palette::field_text()),
                        );
                        self.text_editors.insert(response.id);
                        crate::numeric_tab::register(ui, &response, Some(draft.as_str()));
                        if response.lost_focus() {
                            let name = draft.trim().to_owned();
                            if crate::numeric_tab::accepted(&response) && !name.is_empty() {
                                edited.name = name;
                            }
                            // Pick the stored name up again next frame.
                            self.shading.name_draft = None;
                        } else if !response.has_focus() {
                            *draft = node.name.clone();
                        }
                    }
                });
                // A Material's layer rows sit under Name on its first tab
                // (UIC-104: Name above every tab's sections).
                let layer_rows = !matches!(node.kind, NodeKind::Material(_))
                    || crate::material_layout::tab_for(&self.properties_tab)
                        == crate::material_layout::MATERIAL_REF_TAB;
                if layer_rows && !matches!(node.kind, NodeKind::Folder) {
                    crate::option_row::check(ui, "Enable", &mut edited.layer.enable);
                }
                if layer_rows && !matches!(node.kind, NodeKind::Folder | NodeKind::RenderOutput(_))
                {
                    crate::option_row::check(ui, "Invert", &mut edited.layer.invert);
                    // NM-12: a Node Material blends Normal only for now.
                    let node_material = matches!(node.kind, NodeKind::NodeMaterial(_));
                    ui.gated_ui(
                        crate::unavailable::unless(
                            !node_material,
                            "Node Materials blend Normal only for now; their opacity mixes them",
                        ),
                        |ui| {
                            crate::option_row::field_row(ui, "Blend Mode", |ui| {
                                enum_combo(
                                    ui,
                                    "shader-blend-mode",
                                    BlendMode::ALL,
                                    &mut edited.layer.blend,
                                    |m| m.label(),
                                );
                            });
                        },
                    );
                    crate::option_row::field_row(ui, "Opacity", |ui| {
                        continuing |= percent(ui, &mut edited.layer.opacity, 0.);
                    });
                }
                if let NodeKind::Shader(sh) = &mut edited.kind {
                    continuing |= self.shader_properties(ui, sh);
                }
                if matches!(edited.kind, NodeKind::NodeMaterial(_)) {
                    continuing |= self.node_material_rows(ui, &mut edited);
                }
                self.kind_properties(ui, &mut edited);
                // Gradient (gradient family §1): input, strips, keys.
                if let NodeKind::Texture(t) = &mut edited.kind
                    && let operandi_core::shader_tree::TextureSource::Gradient(g) = &mut t.source
                {
                    let color = t.effect == TextureEffect::Rgba
                        || operandi_core::shader_eval::Ch::of_effect(t.effect)
                            .is_some_and(|c| c.is_color());
                    // NE-03/04: a wire may drive the Input.
                    let driver = self
                        .editor
                        .scene
                        .shader_tree
                        .setting_driver(edited.id, "texture.gradient_input")
                        .map(|(group, w)| (group, w.from.clone()));
                    continuing |= crate::gradient_ui::properties(ui, g, color, driver.is_some());
                    if let Some((group, from)) = driver
                        && ui
                            .small_button(format!("driven by nodes ({from})"))
                            .on_hover_text("Open the node view on the driving node")
                            .clicked()
                    {
                        self.open_group_nodes(group);
                        self.graph_window.board.as_rendered = false;
                        self.graph_window.board.selected = Some(from);
                    }
                    // UIC-03: it opens a window.
                    if ui
                        .button("Edit Gradient…")
                        .on_hover_text(
                            "A larger editor with presets, Reverse and Distribute Evenly",
                        )
                        .clicked()
                    {
                        self.shading.gradient_window = Some(node.id);
                    }
                }
                // Render outputs v1 §2: the output's own settings.
                if let NodeKind::RenderOutput(_) = &edited.kind {
                    self.output_properties(ui, node.id);
                }
                // Group extras §1: the Scale block's button and the
                // per-layer opt-out.
                if let NodeKind::Group(g) = &edited.kind
                    && g.scale.scale != 1.
                    && ui
                        .button("Apply Scale")
                        .on_hover_text(
                            "Bake the Scale into the materials and texture sizes inside, \
                             then set it back to 100%",
                        )
                        .clicked()
                {
                    self.run(Command::ShaderTree(ShaderTreeEdit::ApplyScale {
                        group: node.id,
                    }));
                }
                if let NodeKind::Texture(t) = &mut edited.kind {
                    crate::option_row::check(ui, "Ignore Scale Group", &mut t.ignore_scale_group);
                }
                if let NodeKind::Texture(t) = &mut edited.kind {
                    let color = t.effect == TextureEffect::Rgba
                        || operandi_core::shader_eval::Ch::of_effect(t.effect)
                            .is_some_and(|c| c.is_color());
                    match &mut t.source {
                        operandi_core::shader_tree::TextureSource::Variation(v) => {
                            continuing |= crate::gradient_ui::variation(ui, v, color);
                        }
                        operandi_core::shader_tree::TextureSource::Occlusion(o) => {
                            use operandi_core::process_occlusion::OcclusionType;
                            crate::option_row::field_row(ui, "Type", |ui| {
                                enum_combo(
                                    ui,
                                    "occlusion-type",
                                    &OcclusionType::ALL,
                                    &mut o.kind,
                                    |k| k.label(),
                                );
                            });
                            crate::option_row::field_row(ui, "Occlusion Rays", |ui| {
                                let r = ui.add(
                                    crate::numeric_tab::DragValue::new(&mut o.rays).range(1..=1024),
                                );
                                continuing |= r.dragged() && !r.drag_started();
                            });
                        }
                        operandi_core::shader_tree::TextureSource::Process(_) => {
                            ui.weak("Adjusts the texture layer directly below, in this effect");
                        }
                        operandi_core::shader_tree::TextureSource::VertexMap { map, .. } => {
                            let maps = self.editor.scene.vertex_maps();
                            let name = |m: &operandi_core::vertex_map_texture::VertexMapRef| {
                                use operandi_core::vertex_map_texture::VertexMapKind;
                                match m.kind {
                                    VertexMapKind::Weight => format!("{} (Weight)", m.name),
                                    VertexMapKind::Color => format!("{} (Color)", m.name),
                                }
                            };
                            crate::option_row::field_row(ui, "Vertex Map", |ui| {
                                ComboBox::from_id_salt("vertex-map")
                                    .selected_text(map.as_ref().map_or("(none)".into(), name))
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(map, None, "(none)");
                                        for m in &maps {
                                            ui.selectable_value(map, Some(m.clone()), name(m));
                                        }
                                    });
                            });
                            if map.as_ref().is_some_and(|m| {
                                m.kind == operandi_core::vertex_map_texture::VertexMapKind::Color
                            }) {
                                ui.weak("A colour map is used as it is: the zones are ignored");
                            }
                        }
                        _ => {}
                    }
                }
                self.shading_image_row(ui, &node);
                // A Shader's channels are drawn by section above.
                let sectioned = matches!(edited.kind, NodeKind::Shader(_));
                // CH-Q1: each channel's status light and Reset to Default.
                let base =
                    self.editor.scene.shader_tree.material_for_tag("Default") == Some(node.id);
                let defaults = node
                    .kind
                    .defaults(base)
                    .map(|kind| {
                        let mut fresh = node.clone();
                        fresh.kind = kind;
                        channel_defaults(fresh.channels())
                    })
                    .unwrap_or_default();
                let mut resets = Vec::new();
                // UIC-55: a value picked from a row's presets.
                let mut preset_pick: Option<(&'static str, f64)> = None;
                // UIC-92/93: Material rows grey out in place while the row
                // above them turns them off; a row a texture layer drives
                // (or whose controlling row one drives) stays live.
                let material = match &edited.kind {
                    NodeKind::Material(m) => Some((**m).clone()),
                    _ => None,
                };
                let driven =
                    operandi_core::openpbr::driven_rows(&self.editor.scene.shader_tree, node.id);
                // Material rows sit in tabs and sections (material_layout).
                let material_tab = material
                    .as_ref()
                    .map(|_| crate::material_layout::tab_for(&self.properties_tab));
                let mut shading_model = material.as_ref().map(|m| m.shading_model);
                let mut reflection_type = material.as_ref().map(|m| m.more.reflection_type);
                let value_presets = &mut self.value_presets;
                // NE-04: channels a free node's wire drives, and the
                // marker the user clicked (opens the node view on it).
                let tree_now = self.editor.scene.shader_tree.clone();
                let mut open_driver: Option<(NodeId, String)> = None;
                let mut channel_row = |ui: &mut Ui, name: &'static str, channel: Channel<'_>| {
                    let default = channel_light(&defaults, name, &channel);
                    // A material channel, or a texture layer's colour or
                    // value setting (NE-03/04).
                    let key = match &node.kind {
                        NodeKind::Material(_) => operandi_core::node_view::CHANNELS
                            .iter()
                            .find(|(_, _, label)| *label == name)
                            .map(|(_, k, _)| *k),
                        NodeKind::Texture(t) => operandi_core::node_view_settings::settings(t)
                            .into_iter()
                            .find(|(_, label, _, _)| *label == name)
                            .map(|(k, _, _, _)| k),
                        _ => None,
                    };
                    let wire = key.and_then(|k| {
                        tree_now
                            .setting_driver(node.id, k)
                            .map(|(g, w)| (g, w.from.clone()))
                    });
                    if wire.is_some() {
                        crate::form::rows::set_status(crate::form::status::ChannelStatus::Driven);
                    }
                    let why = if wire.is_some() {
                        Some("Driven by nodes: remove the wire in the node view to edit it")
                    } else {
                        material.as_ref().and_then(|m| {
                            operandi_core::openpbr::inert_reason_driven(m, name, &|row| {
                                driven.contains(&row)
                            })
                        })
                    };
                    ui.gated_ui(why, |ui| {
                        if let Channel::Bool(b) = channel {
                            crate::option_row::check(ui, name, b);
                        } else {
                            crate::option_row::field_row(ui, name, |ui| {
                                // UIC-55: the presets arrow at the field's
                                // far right (the row lays out right to left).
                                if let Some(key) = crate::value_presets::material_key(name)
                                    && let Some(v) = channel_number(&channel)
                                {
                                    // MAT-ED-Q01: Dispersion presets are
                                    // physical B−G figures, shown and
                                    // saved as the material's figure.
                                    let physical = key == crate::value_presets::DISPERSION;
                                    let v = match (&material, physical) {
                                        (Some(m), true) => {
                                            operandi_core::dispersion::figure_of_material(m)
                                        }
                                        _ => v,
                                    };
                                    let format: Box<dyn Fn(f64) -> String> = if physical {
                                        Box::new(trim_number)
                                    } else {
                                        Box::new(preset_format(&channel))
                                    };
                                    let out = crate::value_presets::arrow(
                                        ui,
                                        key,
                                        value_presets,
                                        v,
                                        &format,
                                    );
                                    if let Some(pick) = out.picked {
                                        preset_pick = Some((name, pick));
                                    }
                                    preset_saved |= out.changed;
                                    let rest = ui.available_width();
                                    ui.spacing_mut().interact_size.x = rest;
                                }
                                continuing |= channel_widget_named(ui, name, channel);
                            });
                            if let Some(drag) = crate::colour_field::take_gang() {
                                gangs.push((name, drag));
                            }
                        }
                    });
                    if let Some(driver) = wire
                        && ui
                            .small_button(format!("driven by nodes ({})", driver.1))
                            .on_hover_text("Open the node view on the driving node")
                            .clicked()
                    {
                        open_driver = Some(driver);
                    }
                    if crate::form::rows::take_reset()
                        && let Some(d) = default
                    {
                        resets.push((name, d));
                    }
                };
                let mut channels: Vec<Option<(&'static str, Channel<'_>)>> = edited
                    .channels()
                    .into_iter()
                    .filter(|_| !sectioned)
                    .map(Some)
                    .collect();
                if let Some(tab) = material_tab {
                    use crate::material_layout::{REFLECTION_TYPE, SHADING_MODEL, sections};
                    if sections(tab).is_empty() {
                        crate::form::rows::note(ui, "None yet");
                    }
                    for section in sections(tab) {
                        crate::form::rows::section(ui, section.name, false, |ui| {
                            for &row in section.rows {
                                match row {
                                    SHADING_MODEL => {
                                        if let Some(model) = &mut shading_model {
                                            crate::option_row::field_row(ui, row, |ui| {
                                                enum_combo(
                                                    ui,
                                                    "material-shading-model",
                                                    operandi_core::shader_tree::ShadingModel::ALL,
                                                    model,
                                                    |s| s.label(),
                                                );
                                            });
                                        }
                                    }
                                    REFLECTION_TYPE => {
                                        if let Some(kind) = &mut reflection_type {
                                            crate::option_row::field_row(ui, row, |ui| {
                                                enum_combo(
                                                    ui,
                                                    "material-reflection-type",
                                                    operandi_core::shader_tree::ReflectionType::ALL,
                                                    kind,
                                                    |r| r.label(),
                                                );
                                            });
                                        }
                                    }
                                    _ => {
                                        let found = channels
                                            .iter_mut()
                                            .find(|c| c.as_ref().is_some_and(|(n, _)| *n == row))
                                            .and_then(Option::take);
                                        if let Some((name, channel)) = found {
                                            channel_row(ui, name, channel);
                                        }
                                    }
                                }
                            }
                        });
                    }
                } else {
                    for (name, channel) in channels.into_iter().flatten() {
                        channel_row(ui, name, channel);
                    }
                }
                if let NodeKind::Material(m) = &mut edited.kind {
                    if let Some(model) = shading_model {
                        m.shading_model = model;
                    }
                    if let Some(kind) = reflection_type {
                        m.more.reflection_type = kind;
                    }
                }
                // MAT-ED-Q01 (Jason, 2026-11-12): a Dispersion preset is a
                // physical figure: Dispersion goes to 100 % and Abbe Number
                // follows from the current Refractive Index.
                if let Some((group, driver)) = open_driver {
                    self.open_group_nodes(group);
                    self.graph_window.board.as_rendered = false;
                    self.graph_window.board.selected = Some(driver);
                }
                if let Some(("Dispersion", figure)) = preset_pick
                    && let NodeKind::Material(m) = &mut edited.kind
                {
                    preset_pick = None;
                    if let Err(e) = operandi_core::dispersion::apply_figure(m, figure) {
                        self.status = e.to_string();
                    }
                }
                for (name, channel) in edited.channels() {
                    if let Some((_, d)) = resets.iter().find(|(n, _)| *n == name) {
                        set_channel(channel, d);
                    } else if let Some((_, v)) = preset_pick.filter(|(n, _)| *n == name) {
                        set_channel(channel, &ChannelValue::Number(v));
                    }
                }
            }
        }
        if preset_saved {
            self.save_value_presets();
        }
        if matches!(node.kind, NodeKind::Material(_))
            && crate::material_layout::tab_for(&self.properties_tab)
                == crate::material_layout::MATERIAL_REF_TAB
        {
            ui.weak(
                "Renders use these channels; the viewport still shows the Set Material \
                 values (color, Diffuse, Specular, Smoothing).",
            );
        }
        if edited != node {
            self.replace_layer(edited, continuing);
        }
        // A5/CN-08: the same swatch drag on every other selected layer of
        // this type: relative for Shift/Ctrl/Alt, absolute for a component.
        for (name, drag) in gangs {
            for other in nodes.iter().filter(|n| n.id != node.id) {
                if std::mem::discriminant(&other.kind) != std::mem::discriminant(&node.kind) {
                    continue;
                }
                let mut changed = other.clone();
                for (n, channel) in changed.channels() {
                    if n == name
                        && let Channel::Color(c) = channel
                    {
                        *c = drag.apply(*c);
                    }
                }
                if changed != *other {
                    self.replace_layer(changed, false);
                }
            }
        }
    }

    /// The Texture Locator sub-tab: the layer's locator item's transform and
    /// projection settings. Settings marked "stored" don't render yet
    /// (stage 1).
    fn texture_locator_properties(&mut self, ui: &mut Ui, node: &Node) {
        let NodeKind::Texture(t) = &node.kind else {
            return;
        };
        let Some(item) = t.locator.and_then(|id| {
            self.editor
                .scene
                .items
                .iter()
                .find(|i| i.id == id && i.texture_locator().is_some())
        }) else {
            ui.weak("This layer has no Texture Locator; pick one on the Layer tab");
            return;
        };
        let locator = item.id;
        let settings = item.texture_locator().expect("checked").clone();
        let transform = item.transform.clone();
        ui.weak(item.name.clone());
        let mut s = settings.clone();
        let mut tr = transform.clone();
        // Set while a drag continues, so one drag is one undo step.
        let dragging = std::cell::Cell::new(false);
        let mut auto_size = false;
        const STORED: &str = "Stored with the scene; renders in a later stage";
        let drag = |ui: &mut Ui,
                    v: &mut f64,
                    range: std::ops::RangeInclusive<f64>,
                    speed: f64,
                    suffix: &str| {
            let field = crate::numeric_tab::DragValue::new(v)
                .range(range)
                .speed(speed)
                .max_decimals(4);
            // Metres read units ("25 mm") and show the Distance Units.
            let field = if suffix == " m" {
                field.distance()
            } else {
                field.suffix(suffix.to_owned())
            };
            let r = ui.add(field);
            if r.dragged() && !r.drag_started() {
                dragging.set(true);
            }
        };
        let vector = |ui: &mut Ui,
                      label: &str,
                      v: &mut glam::DVec3,
                      range: std::ops::RangeInclusive<f64>,
                      scale: f64,
                      suffix: &str| {
            // UIC-114: one row per component (issue #6: three fields on one
            // row ran past the panel's edge).
            crate::option_row::vector(ui, label, &["X", "Y", "Z"], "", |ui, k| {
                let mut x = v[k] * scale;
                drag(ui, &mut x, range.clone(), 0.01, suffix);
                if (x / scale - v[k]).abs() > 1e-12 {
                    v[k] = x / scale;
                }
            });
        };
        vector(ui, "Position", &mut tr.translation, -1e9..=1e9, 1., " m");
        vector(
            ui,
            "Rotation",
            &mut tr.rotation,
            -1e6..=1e6,
            180. / std::f64::consts::PI,
            "°",
        );
        rotation_order_row(ui, "locator-rotation-order", &mut tr);
        // Planar spec: Size stops at 0.001 m.
        vector(
            ui,
            "Size",
            &mut tr.scale,
            operandi_core::texture_locator::MIN_SIZE..=1e8,
            1.,
            " m",
        );
        crate::option_row::row(ui, "", |ui| {
            if ui
                .button("Auto Size")
                .on_hover_text(
                    "Centre the locator on the surfaces this layer applies to and size it to them",
                )
                .clicked()
            {
                auto_size = true;
            }
        });
        ui.separator();
        crate::option_row::field_row(ui, "Projection Type", |ui| {
            enum_combo(
                ui,
                "locator-projection",
                Projection::ALL,
                &mut s.projection,
                |p| p.label(),
            );
        });
        if s.projection == Projection::Front
            && self
                .editor
                .scene
                .front_matrix(s.projection_camera)
                .is_none()
        {
            ui.weak("No camera to project from: renders as UV Map");
        }
        ui.gated_ui(
            crate::unavailable::unless(uses_axis(s.projection), "This Projection Type has no axis"),
            |ui| {
                crate::option_row::field_row(ui, "Projection Axis", |ui| {
                    ComboBox::from_id_salt("locator-axis")
                        .selected_text(["X", "Y", "Z"][usize::from(s.axis.min(2))])
                        .show_ui(ui, |ui| {
                            for (k, name) in ["X", "Y", "Z"].into_iter().enumerate() {
                                ui.selectable_value(&mut s.axis, k as u8, name);
                            }
                        });
                });
            },
        );
        for (k, label) in ["Horizontal Repeat", "Vertical Repeat"]
            .into_iter()
            .enumerate()
        {
            crate::option_row::field_row(ui, label, |ui| {
                enum_combo(
                    ui,
                    &format!("locator-tiling-{k}"),
                    Tiling::ALL,
                    &mut s.tiling[k],
                    |t| t.label(),
                );
            });
        }
        for (k, label) in ["Repeat U", "Repeat V"].into_iter().enumerate() {
            crate::option_row::field_row(ui, label, |ui| {
                drag(ui, &mut s.repeat[k], -1e6..=1e6, 0.01, "");
            });
        }
        let uv_maps: BTreeSet<String> = self
            .editor
            .scene
            .items
            .iter()
            .flat_map(|i| i.mesh.uv_maps.keys().cloned())
            .collect();
        crate::option_row::field_row(ui, "UV Map", |ui| {
            ComboBox::from_id_salt("locator-uv-map")
                .selected_text(s.uv_map.clone().unwrap_or_else(|| "(none)".into()))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut s.uv_map, None, "(none)");
                    for name in &uv_maps {
                        ui.selectable_value(&mut s.uv_map, Some(name.clone()), name);
                    }
                });
        });
        crate::option_row::field_row(ui, "UV Rotation", |ui| {
            drag(ui, &mut s.uv_rotation, -1e6..=1e6, 0.5, "°");
        });
        crate::option_row::check(ui, "World Coordinates", &mut s.world_coordinates);
        crate::option_row::check(ui, "World Transforms", &mut s.world_transforms)
            .on_hover_text(STORED);
        ui.separator();
        crate::option_row::field_row(ui, "Front Culling", |ui| {
            enum_combo(
                ui,
                "locator-culling",
                FrontCulling::ALL,
                &mut s.front_culling,
                |c| c.label(),
            );
        })
        .response
        .on_hover_text("Renders with Planar projection");
        crate::option_row::field_row(ui, "Triplanar Blending Hardness", |ui| {
            if percent(ui, &mut s.triplanar_hardness, 0.) {
                dragging.set(true);
            }
        })
        .response
        .on_hover_text("0 % blends softly; 100 % switches hard, like Cubic");
        crate::option_row::field_row(ui, "Random Offsets", |ui| {
            enum_combo(
                ui,
                "locator-random",
                RandomOffsets::ALL,
                &mut s.random_offsets,
                |r| r.label(),
            );
        })
        .response
        .on_hover_text(STORED);
        crate::option_row::field_row(ui, "Texture Offset Strength", |ui| {
            drag(ui, &mut s.random_strength, 0.0..=10.0, 0.01, "");
        })
        .response
        .on_hover_text(STORED);
        crate::option_row::field_row(ui, "Falloff Type", |ui| {
            enum_combo(
                ui,
                "locator-falloff",
                FalloffType::ALL,
                &mut s.falloff_type,
                |f| f.label(),
            );
        })
        .response
        .on_hover_text("Fades the layer with distance from the locator centre");
        crate::option_row::vector(
            ui,
            "Falloff",
            &["X", "Y", "Z"],
            "0 is off; the layer is gone at Size / Falloff from the centre",
            |ui, k| drag(ui, &mut s.falloff[k], 0.0..=1e9, 0.01, ""),
        );
        crate::option_row::field_row(ui, "Tangent Type", |ui| {
            enum_combo(
                ui,
                "locator-tangent",
                TangentType::ALL,
                &mut s.tangent,
                |t| t.label(),
            );
        })
        .response
        .on_hover_text(STORED);
        crate::option_row::check(ui, "Real World Size", &mut s.real_world_size)
            .on_hover_text(STORED);
        crate::option_row::vector(ui, "Real World Size", &["X", "Y"], STORED, |ui, k| {
            drag(ui, &mut s.real_world_size_xy[k], 1e-6..=1e6, 0.01, " m");
        });
        let items: Vec<(u64, String)> = self
            .editor
            .scene
            .items
            .iter()
            .filter(|i| i.is_mesh())
            .map(|i| (i.id, i.name.clone()))
            .collect();
        crate::option_row::field_row(ui, "Particle Source", |ui| {
            let shown = s.particle_source.map_or("(none)".to_owned(), |id| {
                items
                    .iter()
                    .find(|(i, _)| *i == id)
                    .map_or("(missing)".to_owned(), |(_, n)| n.clone())
            });
            ComboBox::from_id_salt("locator-particles")
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut s.particle_source, None, "(none)");
                    for (id, name) in &items {
                        ui.selectable_value(&mut s.particle_source, Some(*id), name);
                    }
                });
        })
        .response
        .on_hover_text(STORED);
        // Stored only: the Texture Replicator.
        crate::option_row::field_row(ui, "Texture Instancer", |ui| {
            item_combo(ui, "locator-replicator", &mut s.replicator, &items);
        })
        .response
        .on_hover_text(STORED);
        // Front's Projection Camera (TL-Q30); greyed for other types
        // (UIC-92).
        let cameras: Vec<(u64, String)> = self
            .editor
            .scene
            .items
            .iter()
            .filter(|i| i.camera.is_some())
            .map(|i| (i.id, i.name.clone()))
            .collect();
        ui.gated_ui(
            crate::unavailable::unless(
                s.projection == Projection::Front,
                "Only Front projects from a camera",
            ),
            |ui| {
                crate::option_row::field_row(ui, "Projection Camera", |ui| {
                    item_combo(ui, "locator-camera", &mut s.projection_camera, &cameras);
                })
                .response
                .on_hover_text("The camera Front projects from; (none) uses the render camera");
            },
        );
        if s != settings || tr != transform {
            self.run(Command::ShaderTree(ShaderTreeEdit::SetLocator {
                locator,
                settings: Box::new(s),
                transform: tr,
                continuing: dragging.get(),
            }));
        }
        if auto_size {
            self.run(Command::ShaderTree(ShaderTreeEdit::AutoSize {
                layer: node.id,
            }));
        }
    }

    /// A Shader's settings by section (shader item v1, §1): each section's
    /// rows grey while its control switch is off (the section then comes
    /// from the shader below), and fog rows grey when the Fog Type doesn't
    /// use them. Returns whether a drag continues.
    fn shader_properties(&self, ui: &mut Ui, s: &mut ShaderChannels) -> bool {
        use operandi_core::camera_light::LinkMode;
        use operandi_core::shader_tree::{AlphaType, FogType, IndirectIllumType};
        let groups: Vec<(u64, String)> = self
            .editor
            .scene
            .groups
            .iter()
            .map(|g| (g.id, g.name.clone()))
            .collect();
        let mut continuing = false;
        // CH-Q1: status lights against a new Shader's values.
        let defaults = channel_defaults(ShaderChannels::default().channels());
        let mut resets = Vec::new();
        for section in ["Shading", "Quality", "Visibility", "Light Linking", "Fog"] {
            ui.add_space(4.);
            ui.strong(section);
            let m = &s.more;
            let on = match section {
                "Shading" => m.compute_shading,
                "Quality" => m.control_quality,
                "Visibility" => m.control_visibility,
                "Light Linking" => m.control_light_linking,
                _ => m.control_fog,
            };
            let fog = m.fog_type;
            let switch = match section {
                "Shading" => "Compute Shading",
                "Quality" => "Control Quality",
                "Visibility" => "Control Visibility",
                "Light Linking" => "Control Light Linking",
                _ => "Control Fog",
            };
            let off = format!("Turn on {switch} first");
            // The menus of the section first.
            ui.gated_ui((!on).then_some(off.as_str()), |ui| match section {
                "Shading" => {
                    crate::option_row::field_row(ui, "Effect", |ui| {
                        enum_combo(ui, "shader-effect", ShaderEffect::ALL, &mut s.effect, |e| {
                            e.label()
                        });
                    });
                }
                "Quality" => {
                    crate::option_row::field_row(ui, "Indirect Illum Type", |ui| {
                        enum_combo(
                            ui,
                            "shader-indirect-type",
                            IndirectIllumType::ALL,
                            &mut s.more.indirect_type,
                            |t| t.label(),
                        );
                    });
                }
                "Visibility" => {
                    crate::option_row::field_row(ui, "Alpha Type", |ui| {
                        enum_combo(
                            ui,
                            "shader-alpha-type",
                            AlphaType::ALL,
                            &mut s.alpha_type,
                            |a| a.label(),
                        );
                    });
                }
                "Light Linking" => {
                    crate::option_row::field_row(ui, "Light Group", |ui| {
                        let shown = s.more.light_group.map_or("(none)".to_owned(), |g| {
                            groups
                                .iter()
                                .find(|x| x.0 == g)
                                .map_or("(missing)".to_owned(), |x| x.1.clone())
                        });
                        ComboBox::from_id_salt("shader-light-group")
                            .selected_text(shown)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut s.more.light_group, None, "(none)");
                                for (g, name) in &groups {
                                    ui.selectable_value(&mut s.more.light_group, Some(*g), name);
                                }
                            });
                    });
                    crate::option_row::field_row(ui, "Mode", |ui| {
                        enum_combo(
                            ui,
                            "shader-link-mode",
                            &LinkMode::ALL,
                            &mut s.more.link_mode,
                            |m| match m {
                                LinkMode::Include => "Include",
                                LinkMode::Exclude => "Exclude",
                            },
                        );
                    });
                }
                _ => {
                    crate::option_row::field_row(ui, "Fog Type", |ui| {
                        enum_combo(
                            ui,
                            "shader-fog-type",
                            FogType::ALL,
                            &mut s.more.fog_type,
                            |f| f.label(),
                        );
                    });
                }
            });
            let alpha_type = s.alpha_type;
            let rows = s
                .sections()
                .into_iter()
                .find(|(name, _)| *name == section)
                .map(|(_, rows)| rows)
                .unwrap_or_default();
            for (name, channel) in rows {
                use crate::unavailable::unless;
                let why = match name {
                    // The switches themselves always work.
                    "Compute Shading"
                    | "Control Quality"
                    | "Control Visibility"
                    | "Control Light Linking"
                    | "Control Fog"
                    | "GL Display" => None,
                    _ if !on => Some(off.as_str()),
                    "Alpha Value" => unless(
                        alpha_type == AlphaType::Constant,
                        "Set Alpha Type to Constant first",
                    ),
                    "Use Environment Color" => unless(
                        fog == FogType::Underwater,
                        "Set Fog Type to Underwater first",
                    ),
                    "Fog Start Distance" | "Fog End Distance" => {
                        unless(fog == FogType::Linear, "Set Fog Type to Linear first")
                    }
                    "Fog Density" => unless(
                        matches!(
                            fog,
                            FogType::Exponential | FogType::Layered | FogType::Underwater
                        ),
                        "Set Fog Type to Exponential, Layered or Underwater first",
                    ),
                    "Fog Color" => unless(fog != FogType::None, "Choose a Fog Type first"),
                    _ => None,
                };
                let default = channel_light(&defaults, name, &channel);
                ui.gated_ui(why, |ui| {
                    if let Channel::Bool(b) = channel {
                        crate::option_row::check(ui, name, b);
                    } else {
                        crate::option_row::field_row(ui, name, |ui| {
                            continuing |= channel_widget(ui, channel);
                        });
                    }
                });
                if crate::form::rows::take_reset()
                    && let Some(d) = default
                {
                    resets.push((name, d));
                }
            }
        }
        for (name, channel) in s.channels() {
            if let Some((_, d)) = resets.iter().find(|(n, _)| *n == name) {
                set_channel(channel, d);
            }
        }
        continuing
    }

    /// A Render Output's settings (render outputs v1 §2), each change one
    /// undo step (a drag one step).
    fn output_properties(&mut self, ui: &mut Ui, id: NodeId) {
        use operandi_core::render_outputs::{Contours, Denoiser, ExposureControl, ToneMap};
        let before = self.editor.scene.shader_tree.output_settings(id);
        let mut s = before.clone();
        let mut continuing = false;
        ui.add_space(4.);
        ui.strong("Output");
        let final_color = self.editor.scene.shader_tree.get(id).is_some_and(|n| {
            matches!(
                n.kind,
                NodeKind::RenderOutput(operandi_core::shader_tree::OutputEffect::FinalColor)
            )
        });
        if final_color {
            // RI-Q05 (provisional RI-B03): Final Color's Denoising is the
            // render's one denoise switch, Open Image Denoise or none.
            let on = self.editor.scene.render.denoise;
            let mut d = if on { Denoiser::Oidn } else { Denoiser::None };
            crate::option_row::field_row(ui, "Denoising", |ui| {
                enum_combo(
                    ui,
                    "output-denoiser",
                    &[Denoiser::None, Denoiser::Oidn],
                    &mut d,
                    |d| d.label(),
                );
            })
            .response
            .on_hover_text("Clean up the finished render's noise (every Final Color output)");
            if (d != Denoiser::None) != on {
                let mut r = self.editor.scene.render.clone();
                r.denoise = !on;
                self.run(Command::SetRenderSettings(r));
                return;
            }
        } else {
            crate::option_row::field_row(ui, "Denoising", |ui| {
                enum_combo(ui, "output-denoiser", Denoiser::ALL, &mut s.denoiser, |d| {
                    d.label()
                });
            });
        }
        crate::option_row::field_row(ui, "Exposure Control", |ui| {
            enum_combo(
                ui,
                "output-exposure",
                ExposureControl::ALL,
                &mut s.exposure,
                |e| e.label(),
            );
        });
        crate::option_row::field_row(ui, "Tone Map Type", |ui| {
            enum_combo(ui, "output-tone-map", ToneMap::ALL, &mut s.tone_map, |t| {
                t.label()
            });
        });
        crate::option_row::field_row(ui, "Contours", |ui| {
            enum_combo(ui, "output-contours", Contours::ALL, &mut s.contours, |c| {
                c.label()
            });
        });
        crate::option_row::field_row(ui, "Occlusion Rays", |ui| {
            let r =
                ui.add(crate::numeric_tab::DragValue::new(&mut s.occlusion_rays).range(1..=4096));
            continuing |= r.dragged() && !r.drag_started();
        });
        // RO-Q04 v2: where a saved render puts this output, and how.
        crate::option_row::field_row(ui, "Output Filename", |ui| {
            let response = ui
                .add(
                    egui::TextEdit::singleline(&mut s.filename)
                        .hint_text("Render N (output name)")
                        .desired_width(160.),
                )
                .on_hover_text(
                    "Saved with the render, relative to its folder; empty uses the render's name",
                );
            crate::numeric_tab::register(ui, &response, Some(s.filename.as_str()));
        });
        crate::option_row::field_row(ui, "Format", |ui| {
            let shown = if s.format.is_empty() {
                "(as the render)".to_owned()
            } else {
                s.format.clone()
            };
            ComboBox::from_id_salt("output-format")
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut s.format, String::new(), "(as the render)");
                    // OpenEXR Multilayer is the whole render, saved from
                    // the render window (RO-Q17).
                    for f in crate::image_viewer::SaveFormat::per_output() {
                        ui.selectable_value(&mut s.format, f.label().to_owned(), f.label());
                    }
                });
        });
        // Group C: the numbers this output writes. Greyed for Cryptomatte,
        // whose ids are always linear (UIC-117, reason in the tooltip).
        let effect = self
            .editor
            .scene
            .shader_tree
            .get(id)
            .and_then(|n| match n.kind {
                NodeKind::RenderOutput(e) => Some(e),
                _ => None,
            });
        let crypto = effect == Some(operandi_core::shader_tree::OutputEffect::Cryptomatte);
        let heat_map = effect == Some(operandi_core::shader_tree::OutputEffect::ShadedAaSamples);
        // Group D: the expression, its presets and its status line.
        self.lpe_rows(ui, effect, &mut s);
        {
            use crate::unavailable::unless;
            use operandi_core::render_outputs::{CryptomatteType, OutputColorspace};
            ui.gated_ui(
                unless(
                    !crypto && !heat_map,
                    if heat_map {
                        "The heat map's colours are written as they are (a data output)"
                    } else {
                        "Cryptomatte's ids and coverage are always linear numbers"
                    },
                ),
                |ui| {
                    crate::option_row::field_row(ui, "Colorspace", |ui| {
                        enum_combo(
                            ui,
                            "output-colorspace",
                            OutputColorspace::ALL,
                            &mut s.colorspace,
                            |c| c.label(),
                        );
                    })
                    .response
                    .on_hover_text(
                        "Default: beauty outputs are sRGB in 8/16-bit files and linear in \
                         float files; data outputs are always linear. Linear or sRGB forces \
                         the numbers written and how the render window shows them.",
                    );
                },
            );
            // Cryptomatte's own rows, greyed for the other types (UIC-117).
            let why = unless(crypto, "Set the Effect to Cryptomatte first");
            ui.gated_ui(why, |ui| {
                crate::option_row::field_row(ui, "Cryptomatte Type", |ui| {
                    enum_combo(
                        ui,
                        "output-cryptomatte-type",
                        CryptomatteType::ALL,
                        &mut s.cryptomatte_type,
                        |t| t.label(),
                    );
                })
                .response
                .on_hover_text("Which ids the matte ranks: objects, materials or assets");
            });
            ui.gated_ui(why, |ui| {
                crate::option_row::field_row(ui, "Cryptomatte Levels", |ui| {
                    let r = ui.add(
                        crate::numeric_tab::DragValue::new(&mut s.cryptomatte_levels)
                            .range(2..=16)
                            .speed(0.1),
                    );
                    continuing |= r.dragged() && !r.drag_started();
                })
                .response
                .on_hover_text("Ids ranked per pixel: 2 to 16, even numbers");
            });
            // Cycles ranks two levels per pass: keep it even.
            s.cryptomatte_levels = (s.cryptomatte_levels.div_ceil(2) * 2).clamp(2, 16);
        }
        for (name, channel) in s.channels() {
            if heat_map && name == "Image Processing" {
                // RO-Q29: the heat map is never processed; shown off and
                // greyed with the reason (UIC-117).
                let why = crate::unavailable::unless(
                    false,
                    "Shaded AA Samples is a diagnostic picture: it is never processed",
                );
                ui.gated_ui(why, |ui| {
                    crate::option_row::check(ui, name, &mut false);
                });
            } else if let Channel::Bool(b) = channel {
                crate::option_row::check(ui, name, b);
            } else {
                crate::option_row::field_row(ui, name, |ui| {
                    continuing |= channel_widget(ui, channel);
                });
            }
        }
        if s != before {
            // A one-setting edit runs item.channel layer:<id> (UIC-164/165);
            // core's SetOutput is used only when it is exactly this change.
            if let Some(line) = output_edit_line(id, &before.channel_json(), &s.channel_json())
                && let Some(Ok(Some(Command::ShaderTree(ShaderTreeEdit::SetOutput {
                    settings: built,
                    ..
                })))) = operandi_core::command_line::command_for_line(&mut self.editor, &line)
                && *built == s
            {
                #[cfg(test)]
                crate::ui_inventory::note_command(&line);
                self.run(Command::ShaderTree(ShaderTreeEdit::SetOutput {
                    node: id,
                    settings: built,
                    continuing,
                }));
                return;
            }
            self.run(Command::ShaderTree(ShaderTreeEdit::SetOutput {
                node: id,
                settings: Box::new(s),
                continuing,
            }));
        }
    }

    /// Type-specific choices that aren't plain channels.
    /// An Image Map's Image: the scene's pictures, or Load Image… (ST-4).
    fn shading_image_row(&mut self, ui: &mut Ui, node: &Node) {
        let NodeKind::Texture(t) = &node.kind else {
            return;
        };
        let operandi_core::shader_tree::TextureSource::ImageMap { image, .. } = t.source else {
            return;
        };
        let images: Vec<(u64, String, String)> = self
            .editor
            .scene
            .images
            .iter()
            .map(|i| (i.id, i.name.clone(), i.path.clone()))
            .collect();
        let shown = image.map_or("(none)".to_owned(), |id| {
            images
                .iter()
                .find(|(i, ..)| *i == id)
                .map_or("(missing)".to_owned(), |(_, n, _)| n.clone())
        });
        // Small pictures beside the names: decoded once on the preset
        // browser's thumbnail threads and kept there (not per frame).
        let thumbs: Vec<Option<egui::TextureHandle>> = images
            .iter()
            .map(|(.., path)| self.image_thumbnail(path))
            .collect();
        let current = image.and_then(|id| images.iter().position(|(i, ..)| *i == id));
        let current_thumb = current.and_then(|k| thumbs[k].clone());
        let current_path = current.map(|k| images[k].2.clone());
        let mut chosen: Option<Option<u64>> = None;
        let mut load = false;
        let mut view = false;
        crate::option_row::field_row(ui, "Image", |ui| {
            let combo = ComboBox::from_id_salt("shader-image")
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    if image_choice(ui, image.is_none(), "(none)", None, false).clicked() {
                        chosen = Some(None);
                    }
                    for ((id, name, _), thumb) in images.iter().zip(&thumbs) {
                        if image_choice(ui, image == Some(*id), name, thumb.as_ref(), true)
                            .clicked()
                        {
                            chosen = Some(Some(*id));
                        }
                    }
                    ui.separator();
                    if ui.button("Load Image…").clicked() {
                        load = true;
                    }
                });
            // The chosen image's picture at the closed field's left (the
            // name reads from the right, UIC-90).
            let rect = combo.response.rect;
            let at = Rect::from_min_size(
                pos2(rect.left() + 3., rect.center().y - THUMB / 2.),
                vec2(THUMB, THUMB),
            );
            if let Some(t) = &current_thumb {
                paint_thumb(ui.painter(), at, t);
            }
            // Double-click: show the image in the Image Viewer.
            if combo.response.double_clicked() && current_path.is_some() {
                view = true;
                egui::Popup::close_all(ui.ctx());
            }
            combo
                .response
                .on_hover_text("Double-click to show the image in the Image Viewer");
        });
        // UIC-116: the field's button on the row under it (#56: the only
        // way to a file panel was the last entry of the Image list).
        if crate::option_row::button(ui, "Browse…")
            .on_hover_text("Pick a picture file for this layer")
            .clicked()
        {
            // UIC-164: the button runs its command.
            self.run_command_line(&format!("operandi.texture.loadImage layer:{}", node.id));
        }
        if view && let Some(path) = current_path {
            self.show_image_in_viewer(std::path::Path::new(&path));
        }
        if let Some(choice) = chosen
            && choice != image
        {
            let picked = choice.and_then(|id| {
                self.editor
                    .scene
                    .images
                    .iter()
                    .find(|i| i.id == id)
                    .map(|i| (i.path.clone(), i.width, i.height))
            });
            // Runs texture.setIMap (UIC-164/165) when the line builds this
            // exact edit.
            let edit = ShaderTreeEdit::SetImage {
                node: node.id,
                image: picked,
            };
            let name = choice.and_then(|id| {
                self.editor
                    .scene
                    .images
                    .iter()
                    .find(|i| i.id == id)
                    .map(|i| i.name.clone())
            });
            let line = format!(
                "texture.setIMap image:{{{}}} layer:{}",
                name.as_deref().unwrap_or("none"),
                node.id
            );
            match operandi_core::command_line::command_for_line(&mut self.editor, &line) {
                Some(Ok(Some(Command::ShaderTree(built)))) if built == edit => {
                    #[cfg(test)]
                    crate::ui_inventory::note_command(&line);
                    self.run(Command::ShaderTree(built));
                }
                _ => self.run(Command::ShaderTree(edit)),
            }
        }
        if load {
            self.choose_shader_image(node.id);
        }
        #[cfg(feature = "painting")]
        self.psd_layer_row(ui, node.id);
    }

    /// Opens the picture file panel for Image Map layer `node`, in the
    /// folder of the scene's last image.
    fn choose_shader_image(&mut self, node: NodeId) {
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
            .unwrap_or_default();
        if !self.choose(
            crate::file_chooser::Choose::Image { folder },
            crate::file_chooser::Purpose::ShaderImage(node),
        ) {
            self.status = "No file panel is available to choose a picture".into();
        }
    }

    /// `operandi.texture.loadImage layer:ID [filename:PATH]` (#56): the
    /// Image Map's Browse… button runs it (UIC-164). `args` are the
    /// line's words.
    pub(super) fn texture_load_image_canonical(&mut self, args: &[String]) -> bool {
        const NAME: &str = "operandi.texture.loadImage";
        if !args.first().is_some_and(|a| a.eq_ignore_ascii_case(NAME)) {
            return false;
        }
        let bound =
            match operandi_core::command_args::bind(NAME, &args[1..], &["layer", "filename"]) {
                Ok(b) => b,
                Err(e) => {
                    self.status = e;
                    return true;
                }
            };
        let layer = bound[0]
            .and_then(|v| v.trim().parse::<NodeId>().ok())
            .filter(|id| {
                matches!(
                    self.editor.scene.shader_tree.get(*id).map(|n| &n.kind),
                    Some(NodeKind::Texture(t)) if matches!(
                        t.source,
                        operandi_core::shader_tree::TextureSource::ImageMap { .. }
                    )
                )
            });
        let Some(layer) = layer else {
            self.status = format!("{NAME}: layer: takes an Image Map layer's id");
            return true;
        };
        match bound[1] {
            Some(path) => self.shader_image_chosen(layer, std::path::PathBuf::from(path)),
            None => self.choose_shader_image(layer),
        }
        true
    }

    /// A picture file chosen for Image Map layer `node`.
    pub(super) fn shader_image_chosen(&mut self, node: NodeId, path: std::path::PathBuf) {
        match crate::backdrop_ui::decode(&path) {
            Ok((_, (width, height))) => {
                let path = path.to_string_lossy().into_owned();
                if self.run_ok(Command::ShaderTree(ShaderTreeEdit::SetImage {
                    node,
                    image: Some((path, width, height)),
                })) {
                    self.status = format!("Image Map picture {width} × {height} px");
                }
            }
            Err(error) => self.status = error,
        }
    }

    fn kind_properties(&self, ui: &mut Ui, node: &mut Node) {
        let items: Vec<(u64, String)> = self
            .editor
            .scene
            .items
            .iter()
            .filter(|i| i.is_mesh())
            .map(|i| (i.id, i.name.clone()))
            .collect();
        let materials = self.editor.scene.material_names();
        let parts = self
            .editor
            .scene
            .polygon_tag_names(operandi_core::polygon_tags::PolygonTagKind::Part);
        let sets = self
            .editor
            .scene
            .polygon_tag_names(operandi_core::polygon_tags::PolygonTagKind::SelectionSet);
        match &mut node.kind {
            NodeKind::RenderOutput(effect) => {
                crate::option_row::field_row(ui, "Effect", |ui| {
                    enum_combo(ui, "shader-output-effect", OutputEffect::ALL, effect, |e| {
                        e.label()
                    });
                });
            }
            // Drawn by `shader_properties`.
            NodeKind::Shader(_) => {}
            // Drawn in its section (material_layout).
            NodeKind::Material(_) => {}
            NodeKind::Group(g) => {
                crate::option_row::field_row(ui, "Item", |ui| {
                    let shown = match g.item {
                        ItemMask::All => "(all)".to_owned(),
                        ItemMask::Item(id) | ItemMask::ItemGroup(id) => items
                            .iter()
                            .find(|(i, _)| *i == id)
                            .map_or("(missing)".to_owned(), |(_, n)| n.clone()),
                    };
                    ComboBox::from_id_salt("shader-group-item")
                        .selected_text(shown)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut g.item, ItemMask::All, "(all)");
                            for (id, name) in &items {
                                ui.selectable_value(&mut g.item, ItemMask::Item(*id), name);
                            }
                        });
                });
                crate::option_row::field_row(ui, "Polygon Tag Type", |ui| {
                    let tag_type = g.tag_type;
                    enum_combo(
                        ui,
                        "shader-group-tag-type",
                        TagType::ALL,
                        &mut g.tag_type,
                        |t| t.label(),
                    );
                    // A new tag type starts at "(none)" (v0.1).
                    if g.tag_type != tag_type {
                        g.tag = TagMask::None;
                    }
                });
                crate::option_row::field_row(ui, "Polygon Tag", |ui| {
                    // The scene's tags of this type (G-01: selection sets
                    // from every mesh's polygons).
                    let names = match g.tag_type {
                        TagType::Material => &materials,
                        TagType::Part => &parts,
                        TagType::SelectionSet => &sets,
                    };
                    ComboBox::from_id_salt("shader-group-tag")
                        .selected_text(g.tag.label().to_owned())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut g.tag, TagMask::All, "(all)");
                            ui.selectable_value(&mut g.tag, TagMask::None, "(none)");
                            for name in names {
                                ui.selectable_value(&mut g.tag, TagMask::Name(name.clone()), name);
                            }
                        });
                });
                crate::option_row::field_row(ui, "Scope", |ui| {
                    enum_combo(
                        ui,
                        "shader-group-scope",
                        operandi_core::shader_tree::Scope::ALL,
                        &mut g.scope,
                        |s| s.label(),
                    );
                });
            }
            // Drawn by `node_material_rows`.
            NodeKind::Folder | NodeKind::NodeMaterial(_) => {}
            NodeKind::Texture(t) => {
                // NM-13: the Node Inputs heading lists the exposed inputs
                // of the Node Materials below.
                let inputs = self.editor.scene.shader_tree.node_inputs_for(node.id);
                crate::option_row::field_row(ui, "Effect", |ui| {
                    let shown = t
                        .node_input
                        .as_deref()
                        .cloned()
                        .unwrap_or_else(|| t.effect.label().to_owned());
                    ComboBox::from_id_salt("shader-texture-effect")
                        .selected_text(shown)
                        .height(400.)
                        .show_ui(ui, |ui| {
                            for (section, effects) in TextureEffect::MENU {
                                ui.weak(*section);
                                for &e in *effects {
                                    let on = t.node_input.is_none() && t.effect == e;
                                    if ui.selectable_label(on, e.label()).clicked() {
                                        t.effect = e;
                                        t.node_input = None;
                                    }
                                }
                            }
                            if !inputs.is_empty() {
                                ui.weak("Node Inputs");
                                for e in &inputs {
                                    let on = t.node_input.as_deref().map(String::as_str)
                                        == Some(e.name.as_str());
                                    if ui.selectable_label(on, &e.name).clicked() {
                                        t.node_input = Some(Box::new(e.name.clone()));
                                    }
                                }
                            }
                        });
                });
                if let operandi_core::shader_tree::TextureSource::ImageMap { settings, .. } =
                    &mut t.source
                {
                    use operandi_core::shader_tree::{Filtering, ImageAlpha, Swizzle};
                    crate::option_row::field_row(ui, "Swizzling", |ui| {
                        enum_combo(
                            ui,
                            "shader-swizzle",
                            Swizzle::ALL,
                            &mut settings.swizzle,
                            |s| s.label(),
                        );
                    });
                    crate::option_row::field_row(ui, "Alpha", |ui| {
                        enum_combo(
                            ui,
                            "shader-image-alpha",
                            ImageAlpha::ALL,
                            &mut settings.alpha,
                            |a| a.label(),
                        );
                    });
                    crate::option_row::field_row(ui, "Filtering", |ui| {
                        enum_combo(
                            ui,
                            "shader-filtering",
                            Filtering::ALL,
                            &mut settings.filtering,
                            |f| f.label(),
                        );
                    });
                }
                if let operandi_core::shader_tree::TextureSource::Procedural(p) = &mut t.source {
                    procedural_type_rows(ui, p);
                }
                // Locator: the Texture Locator item placing this layer;
                // several layers may share one.
                let locators: Vec<(u64, String)> = self
                    .editor
                    .scene
                    .items
                    .iter()
                    .filter(|i| i.texture_locator().is_some())
                    .map(|i| (i.id, i.name.clone()))
                    .collect();
                crate::option_row::field_row(ui, "Locator", |ui| {
                    let shown = t.locator.map_or("(none)".to_owned(), |id| {
                        locators
                            .iter()
                            .find(|(i, _)| *i == id)
                            .map_or("(missing)".to_owned(), |(_, n)| n.clone())
                    });
                    ComboBox::from_id_salt("shader-texture-locator")
                        .selected_text(shown)
                        .show_ui(ui, |ui| {
                            for (id, name) in &locators {
                                ui.selectable_value(&mut t.locator, Some(*id), name);
                            }
                        });
                });
            }
        }
    }
}

/// A layer's type icon (generator batch 19b, in place of letter badges).
fn type_icon(node: &Node) -> crate::icons::Icon {
    use crate::icons::Icon;
    match &node.kind {
        NodeKind::RenderOutput(_) => Icon::TreeOutput,
        NodeKind::Shader(_) => Icon::TreeShader,
        NodeKind::Material(_) => Icon::TreeMaterial,
        NodeKind::Group(_) => Icon::TreeGroup,
        NodeKind::Folder if node.extras.reference.is_some() => Icon::TreeReference,
        NodeKind::Folder => Icon::TreeFolder,
        NodeKind::Texture(_) => Icon::TreeTexture,
        NodeKind::NodeMaterial(_) => Icon::TreeNodeMaterial,
    }
}

/// A procedural layer's choices that aren't plain channels: its type, and
/// for Noise the number of octaves and Flow Noise.
fn procedural_type_rows(ui: &mut Ui, p: &mut operandi_core::procedural::Procedural) {
    use operandi_core::procedural::{
        CellularType, CheckerType, DotsType, GridType, MAX_OCTAVES, MAX_SOURCES, NoiseType, Pattern,
    };
    match &mut p.pattern {
        Pattern::Checker { kind, .. } => {
            crate::option_row::field_row(ui, "Type", |ui| {
                enum_combo(ui, "procedural-type", CheckerType::ALL, kind, |k| k.label());
            });
        }
        Pattern::Grid { kind, .. } => {
            crate::option_row::field_row(ui, "Type", |ui| {
                enum_combo(ui, "procedural-type", GridType::ALL, kind, |k| k.label());
            });
        }
        Pattern::Dots { kind, .. } => {
            crate::option_row::field_row(ui, "Type", |ui| {
                enum_combo(ui, "procedural-type", DotsType::ALL, kind, |k| k.label());
            });
        }
        Pattern::Noise(n) => {
            crate::option_row::field_row(ui, "Type", |ui| {
                enum_combo(ui, "procedural-type", NoiseType::ALL, &mut n.kind, |k| {
                    k.label()
                });
            });
            ui.gated_ui(
                crate::unavailable::unless(
                    n.kind != NoiseType::Simple,
                    "Simple noise has one frequency; choose another Type",
                ),
                |ui| {
                    crate::option_row::field_row(ui, "Frequencies", |ui| {
                        ui.add(
                            crate::numeric_tab::DragValue::new(&mut n.frequencies)
                                .range(1..=MAX_OCTAVES)
                                .speed(0.05),
                        );
                    });
                },
            );
            crate::option_row::check(ui, "Flow Noise", &mut n.flow);
        }
        Pattern::Cellular {
            kind, frequencies, ..
        } => {
            crate::option_row::field_row(ui, "Type", |ui| {
                enum_combo(ui, "procedural-type", CellularType::ALL, kind, |k| {
                    k.label()
                });
            });
            crate::option_row::field_row(ui, "Frequencies", |ui| {
                ui.add(
                    crate::numeric_tab::DragValue::new(frequencies)
                        .range(1..=MAX_OCTAVES)
                        .speed(0.05),
                );
            });
        }
        Pattern::Ripples { sources, .. } => {
            crate::option_row::field_row(ui, "Sources", |ui| {
                ui.add(
                    crate::numeric_tab::DragValue::new(sources)
                        .range(1..=MAX_SOURCES)
                        .speed(0.1),
                );
            });
        }
        Pattern::Poisson {
            kind,
            level,
            frequencies,
            solid,
            profile_gradient,
            ..
        } => {
            crate::option_row::field_row(ui, "Type", |ui| {
                enum_combo(ui, "procedural-type", CellularType::ALL, kind, |k| {
                    k.label()
                });
            });
            crate::option_row::field_row(ui, "Level", |ui| {
                ui.add(
                    crate::numeric_tab::DragValue::new(level)
                        .range(1..=operandi_core::procedural::MAX_LEVEL)
                        .speed(0.05),
                );
            });
            crate::option_row::field_row(ui, "Frequencies", |ui| {
                ui.add(
                    crate::numeric_tab::DragValue::new(frequencies)
                        .range(1..=MAX_OCTAVES)
                        .speed(0.05),
                );
            });
            crate::option_row::check(ui, "Solid 3D Cells", solid);
            crate::option_row::check(ui, "Use Profile Gradient", profile_gradient);
        }
        Pattern::Wood { .. } | Pattern::Weave { .. } | Pattern::Gabor { .. } => {}
    }
}

/// The B column is narrow: blank for Normal (Q-30), else a short name.
fn short_blend(mode: BlendMode) -> &'static str {
    match mode {
        BlendMode::Normal => "",
        BlendMode::Add => "Add",
        BlendMode::Subtract => "Sub",
        BlendMode::Difference => "Diff",
        BlendMode::NormalMultiply => "NMul",
        BlendMode::Divide => "Div",
        BlendMode::Multiply => "Mul",
        BlendMode::Screen => "Scr",
        BlendMode::Overlay => "Ovr",
        BlendMode::SoftLight => "Soft",
        BlendMode::HardLight => "Hard",
        BlendMode::Lighten => "Ltn",
        BlendMode::Darken => "Drk",
        BlendMode::ColorDodge => "Dodg",
        BlendMode::ColorBurn => "Burn",
        BlendMode::NormalMapBlend => "NMap",
    }
}

/// The Effect menu for one row; returns the edited layer when one is picked.
fn effect_menu(
    ui: &mut Ui,
    node: &Node,
    inputs: &[operandi_core::node_graph::Exposed],
) -> Option<Node> {
    let mut edited = node.clone();
    match &mut edited.kind {
        NodeKind::Texture(t) => {
            for (section, effects) in TextureEffect::MENU {
                ui.menu_button(*section, |ui| {
                    for &e in *effects {
                        let on = t.node_input.is_none() && t.effect == e;
                        if ui.selectable_label(on, e.label()).clicked() {
                            t.effect = e;
                            t.node_input = None;
                        }
                    }
                });
            }
            // NM-13: the exposed inputs of the Node Materials below.
            if !inputs.is_empty() {
                ui.menu_button("Node Inputs", |ui| {
                    for e in inputs {
                        let on =
                            t.node_input.as_deref().map(String::as_str) == Some(e.name.as_str());
                        if ui.selectable_label(on, &e.name).clicked() {
                            t.node_input = Some(Box::new(e.name.clone()));
                        }
                    }
                });
            } else if let Some(name) = &t.node_input {
                ui.separator();
                ui.weak(format!("Drives “{name}”: no Node Material below has it"));
            }
        }
        NodeKind::Shader(s) => {
            for &e in ShaderEffect::ALL {
                if ui.selectable_label(s.effect == e, e.label()).clicked() {
                    s.effect = e;
                }
            }
        }
        NodeKind::RenderOutput(effect) => {
            for &e in OutputEffect::ALL {
                if ui.selectable_label(*effect == e, e.label()).clicked() {
                    *effect = e;
                }
            }
        }
        _ => {}
    }
    (edited != *node).then_some(edited)
}

/// A Texture Locator's Rotation Order row: all six orders (TL-Q09,
/// env-texture-locator). Changing it keeps the angles (IT-PHY-01).
pub(super) fn rotation_order_row(ui: &mut Ui, id: &str, tr: &mut operandi_core::scene::Transform) {
    use operandi_core::scene::RotationOrder;
    let mut order = tr.rotation_order();
    crate::option_row::field_row(ui, "Rotation Order", |ui| {
        enum_combo(ui, id, &RotationOrder::ALL, &mut order, |o| o.label());
    })
    .response
    .on_hover_text("The order the three angles turn in; changing it keeps the angles");
    if order != tr.rotation_order() {
        tr.set_rotation_order(order);
    }
}

pub(super) fn enum_combo<T: Copy + PartialEq>(
    ui: &mut Ui,
    id: &str,
    all: &[T],
    value: &mut T,
    label: impl Fn(T) -> &'static str,
) {
    ComboBox::from_id_salt(id)
        .selected_text(label(*value))
        .show_ui(ui, |ui| {
            for &v in all {
                ui.selectable_value(value, v, label(v));
            }
        });
}

/// A 0–100% drag field over a 0–1 value. True while a drag continues.
pub(super) fn percent(ui: &mut Ui, value: &mut f64, low: f64) -> bool {
    let response = crate::value_display::factor(ui, value, low, 1.);
    response.dragged() && !response.drag_started()
}

/// CH-Q1: a channel's value, owned, to compare with its default.
fn channel_value(c: &Channel<'_>) -> ChannelValue {
    match c {
        Channel::Percent(v)
        | Channel::PercentIn(v, ..)
        | Channel::Distance(v)
        | Channel::Angle(v)
        | Channel::Number(v, ..) => ChannelValue::Number(**v),
        Channel::Color(c) => ChannelValue::Vector(**c),
        Channel::Bool(b) => ChannelValue::Bool(**b),
    }
}

/// Side of an image thumbnail in the Image list and field.
const THUMB: f32 = 24.;

/// A picture fitted into `rect`, keeping its shape.
fn paint_thumb(painter: &egui::Painter, rect: Rect, texture: &egui::TextureHandle) {
    let size = texture.size_vec2();
    let fit = rect.width() / size.x.max(size.y).max(1.);
    let shown = Rect::from_center_size(rect.center(), size * fit);
    painter.image(
        texture.id(),
        shown,
        Rect::from_min_max(pos2(0., 0.), pos2(1., 1.)),
        Color32::WHITE,
    );
}

/// One entry of the Image list: its thumbnail (or an empty square while
/// it decodes) and its name. `picture`: whether the entry has one.
fn image_choice(
    ui: &mut Ui,
    selected: bool,
    name: &str,
    thumb: Option<&egui::TextureHandle>,
    picture: bool,
) -> egui::Response {
    let width = ui.available_width().max(180.);
    let (rect, response) = ui.allocate_exact_size(vec2(width, THUMB + 4.), Sense::click());
    let visuals = ui.visuals();
    if selected {
        ui.painter()
            .rect_filled(rect, 2., visuals.selection.bg_fill);
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, 2., visuals.widgets.hovered.weak_bg_fill);
    }
    let square = Rect::from_min_size(rect.min + vec2(4., 2.), vec2(THUMB, THUMB));
    match thumb {
        Some(t) => paint_thumb(ui.painter(), square, t),
        None if picture => {
            ui.painter()
                .rect_filled(square, 2., visuals.extreme_bg_color);
        }
        None => {}
    }
    let color = if selected {
        visuals.selection.stroke.color
    } else {
        visuals.text_color()
    };
    ui.painter().text(
        pos2(square.right() + 6., rect.center().y),
        egui::Align2::LEFT_CENTER,
        name,
        egui::TextStyle::Body.resolve(ui.style()),
        color,
    );
    response
}

/// A number channel's value (None for colours and switches).
fn channel_number(c: &Channel<'_>) -> Option<f64> {
    match channel_value(c) {
        ChannelValue::Number(v) => Some(v),
        _ => None,
    }
}

/// How a preset list shows a value: as the channel's field does.
fn preset_format(c: &Channel<'_>) -> impl Fn(f64) -> String + use<> {
    let percent = matches!(c, Channel::Percent(_) | Channel::PercentIn(..));
    move |v: f64| {
        if percent && crate::value_display::factor_is_percent() {
            trim_number(v * 100.) + " %"
        } else {
            trim_number(v)
        }
    }
}

/// Up to four decimals, trailing zeros trimmed.
fn trim_number(v: f64) -> String {
    let text = format!("{v:.4}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

impl Operandi {
    /// A scene image's thumbnail if decoded; asks for it otherwise (the
    /// preset browser's thumbnail cache: memory, then disk).
    pub(crate) fn image_thumbnail(&mut self, path: &str) -> Option<egui::TextureHandle> {
        if path.is_empty() {
            return None;
        }
        let job = crate::preset_thumbs::Job {
            key: std::path::PathBuf::from(path),
            kind: operandi_core::preset::library::EntryKind::Image,
            set: None,
            embed: false,
        };
        self.preset_browser.thumbs.get(&job)
    }

    /// The picture file of layer `id`'s image, if it has one.
    pub(crate) fn layer_image_path(&self, id: NodeId) -> Option<std::path::PathBuf> {
        let image = self.editor.scene.shader_tree.layer_image(id)?;
        let entry = self.editor.scene.images.iter().find(|i| i.id == image)?;
        (!entry.path.is_empty()).then(|| std::path::PathBuf::from(&entry.path))
    }

    /// Write the value presets (UIC-55) to the data folder.
    pub(crate) fn save_value_presets(&mut self) {
        if let Err(e) = self
            .value_presets
            .save(&self.environment.paths().value_presets())
        {
            self.status = format!("Couldn't save the value presets: {e}");
        }
    }
}

/// Put `value` back into the channel (Reset to Default).
fn set_channel(c: Channel<'_>, value: &ChannelValue) {
    match (c, value) {
        (
            Channel::Percent(v)
            | Channel::PercentIn(v, ..)
            | Channel::Distance(v)
            | Channel::Angle(v)
            | Channel::Number(v, ..),
            ChannelValue::Number(d),
        ) => *v = *d,
        (Channel::Color(c), ChannelValue::Vector(d)) => *c = *d,
        (Channel::Bool(b), ChannelValue::Bool(d)) => *b = *d,
        _ => {}
    }
}

/// Every channel's default, by name.
fn channel_defaults(
    channels: Vec<(&'static str, Channel<'_>)>,
) -> Vec<(&'static str, ChannelValue)> {
    channels
        .into_iter()
        .map(|(name, c)| (name, channel_value(&c)))
        .collect()
}

/// Before a channel row: its status light against its default (UIC-112).
/// Returns the default, for a Reset to Default chosen on the row.
fn channel_light(
    defaults: &[(&'static str, ChannelValue)],
    name: &str,
    channel: &Channel<'_>,
) -> Option<ChannelValue> {
    let default = defaults
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, v)| v.clone())?;
    crate::form::rows::status(channel_value(channel) != default);
    Some(default)
}

/// One channel's widget. True while a drag continues (folds undo steps).
/// [`channel_widget`] with the units a row's name implies (UIC-90): the
/// thin film is in micrometres.
fn channel_widget_named(ui: &mut Ui, name: &str, channel: Channel<'_>) -> bool {
    match (name, channel) {
        ("Thin Film Thickness", Channel::Number(v, lo, hi)) => {
            let response = ui.add(
                crate::numeric_tab::DragValue::new(v)
                    .range(lo..=hi)
                    .speed(0.005)
                    .max_decimals(4)
                    .suffix(" µm"),
            );
            response.dragged() && !response.drag_started()
        }
        (_, channel) => channel_widget(ui, channel),
    }
}

pub(super) fn channel_widget(ui: &mut Ui, channel: Channel<'_>) -> bool {
    let drag =
        |ui: &mut Ui, value: &mut f64, range: std::ops::RangeInclusive<f64>, speed, suffix| {
            let field = crate::numeric_tab::DragValue::new(value)
                .range(range)
                .speed(speed)
                .max_decimals(4);
            let response = ui.add(field.suffix(suffix));
            response.dragged() && !response.drag_started()
        };
    match channel {
        Channel::Percent(v) => percent(ui, v, 0.),
        Channel::PercentIn(v, lo, hi) => {
            let response = crate::value_display::factor(ui, v, lo, hi);
            response.dragged() && !response.drag_started()
        }
        Channel::Color(c) => {
            // Undo folds by the field's gesture (History::undo_fold).
            crate::colour_field::show(ui, c, crate::colour_field::Options::scene());
            false
        }
        // Metres, typed with units and arithmetic (UIC-52): "25 mm".
        Channel::Distance(v) => {
            let response = ui.add(
                crate::numeric_tab::DragValue::new(v)
                    .range(0.0..=1e6)
                    .speed(0.001)
                    .max_decimals(4)
                    .distance(),
            );
            response.dragged() && !response.drag_started()
        }
        Channel::Angle(v) => drag(ui, v, 0.0..=180.0, 0.5, "°"),
        Channel::Number(v, lo, hi) => drag(ui, v, lo..=hi, 0.01, ""),
        Channel::Bool(b) => {
            ui.checkbox(b, "");
            false
        }
    }
}

/// Scene image commands (UIC-160..164): `clip.addStill` (the reference
/// name map's, UIC-165) loads a picture as an image entry; `clip.reload` (old `image.reload`) re-reads a picture
/// file for the viewport (its texture and average colour); `image.list`
/// reports the scene's images, as the Image Map rows show them. Names are
/// Operandi's own (`<domain>.<verb>`): the reference name map has no entry
/// for these (VIT-Q05).
pub(crate) const IMAGE_COMMANDS: [(&str, &str); 3] = [
    (
        "clip.addStill",
        "Help: clip.addStill [filename:]<file path> loads a picture as a scene image entry without showing it anywhere (what dropping a picture outside a 3D view does, BD-60). One undo step.",
    ),
    (
        "clip.reload",
        "Help: clip.reload [image name|file path|all] (old spelling image.reload; bare reloads all) reads the picture file again (Image Map layer menu > Reload Image) and updates the viewport. Changes no scene data, so there is nothing to undo.",
    ),
    (
        "image.list",
        "Help: image.list reports the scene's images: name, file path and pixel size. No arguments.",
    ),
];

/// Node Material commands (Nodes N1). Placeholder names under
/// `operandi.*` until the command registry names them (spec, NM).
pub(crate) const NODE_COMMANDS: [(&str, &str); 3] = [
    (
        "operandi.nodeMaterial.import",
        "Help: operandi.nodeMaterial.import [filename:]<file> [layer:<id>] imports a node graph file (JSON) as Node Material layers, placed as Add Layer places one next to the layer. All or nothing: a graph with a node Operandi can't render is refused. One undo step.",
    ),
    (
        "operandi.nodeMaterial.showGraph",
        "Help: operandi.nodeMaterial.showGraph [layer:<id> | graph:<id>] shows the graph read-only in the Node View panel (docked under the views if it isn't open); bare, the selected Node Material layer's. A Material Group's layer id (or a selected group) shows that material's own MaterialX graph, the one renders use (Show as Nodes). Needs the window; changes nothing, so there is nothing to undo.",
    ),
    (
        "operandi.nodeMaterial.setDefault",
        "Help: operandi.nodeMaterial.setDefault graph:<id> input:<name> value:<v> sets a graph's shared exposed default (a number, or r,g,b with 0..1 components; value:? reads it). Layers without their own value follow. One undo step.",
    ),
];

impl Operandi {
    /// Run a Node Material command line, or its help topic.
    fn node_canonical(&mut self, text: &str) -> bool {
        let Ok(args) = crate::command::tokens(text) else {
            return false;
        };
        let Some(first) = args.first().map(|a| a.to_ascii_lowercase()) else {
            return false;
        };
        let help = |name: &str| {
            NODE_COMMANDS
                .iter()
                .find(|(c, _)| c.eq_ignore_ascii_case(name))
                .map(|(_, h)| *h)
        };
        if first == "help" && args.len() == 2 {
            if let Some(h) = help(&args[1]) {
                self.status = h.into();
                return true;
            }
            return false;
        }
        let Some(usage) = help(&first) else {
            return false;
        };
        let usage = usage.trim_start_matches("Help: ").to_owned();
        // key:value arguments, else positional.
        let named = |key: &str| {
            args[1..].iter().find_map(|a| {
                a.split_once(':')
                    .filter(|(k, _)| k.eq_ignore_ascii_case(key))
                    .map(|(_, v)| v.to_owned())
            })
        };
        let positional: Vec<&String> = args[1..]
            .iter()
            .filter(|a| !a.contains(':') || std::path::Path::new(a).is_absolute())
            .collect();
        match first.as_str() {
            "operandi.nodematerial.import" => {
                let Some(path) =
                    named("filename").or_else(|| positional.first().map(|s| s.to_string()))
                else {
                    self.status = usage;
                    return true;
                };
                let layer = named("layer").and_then(|v| v.parse().ok());
                self.import_node_graph_from(std::path::Path::new(&path), layer);
            }
            "operandi.nodematerial.showgraph" => {
                let tree = &self.editor.scene.shader_tree;
                // A Material Group (named, else selected) shows as nodes.
                let group = named("layer")
                    .and_then(|v| v.parse::<NodeId>().ok())
                    .or_else(|| self.shading.selected.last().copied())
                    .filter(|&g| matches!(tree.get(g).map(|n| &n.kind), Some(NodeKind::Group(_))));
                if let Some(g) = group.filter(|_| named("graph").is_none()) {
                    self.open_group_nodes(g);
                    return true;
                }
                // Bare: the selected Node Material layer.
                let layer: Option<NodeId> =
                    named("layer").and_then(|v| v.parse().ok()).or_else(|| {
                        self.shading.selected.last().copied().filter(|&l| {
                            matches!(
                                tree.get(l).map(|n| &n.kind),
                                Some(NodeKind::NodeMaterial(_))
                            )
                        })
                    });
                let graph = named("graph")
                    .and_then(|v| v.parse().ok())
                    .or_else(|| {
                        layer.and_then(|l| match tree.get(l).map(|n| &n.kind) {
                            Some(NodeKind::NodeMaterial(m)) => m.graph,
                            _ => None,
                        })
                    })
                    .or(self.shading.graph);
                match graph.filter(|g| tree.node_graphs.get(*g).is_some()) {
                    Some(g) => self.open_graph_window(g, layer),
                    None => self.status = usage,
                }
            }
            "operandi.nodematerial.setdefault" => {
                let pick = |key: &str, k: usize| {
                    named(key).or_else(|| positional.get(k).map(|s| s.to_string()))
                };
                let (Some(graph), Some(input), Some(value)) = (
                    pick("graph", 0).and_then(|v| v.parse::<u64>().ok()),
                    pick("input", 1),
                    pick("value", 2),
                ) else {
                    self.status = usage;
                    return true;
                };
                let exposed = self
                    .editor
                    .scene
                    .shader_tree
                    .node_graphs
                    .get(graph)
                    .and_then(|g| g.exposed(&input))
                    .cloned();
                let Some(exposed) = exposed else {
                    self.status = format!("Graph {graph} has no exposed input “{input}”");
                    return true;
                };
                if value.trim() == "?" {
                    self.status = exposed
                        .default_value()
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(",");
                    return true;
                }
                let parsed: std::result::Result<Vec<f64>, std::num::ParseFloatError> =
                    value.split(',').map(|v| v.trim().parse::<f64>()).collect();
                match parsed {
                    Ok(value) => {
                        self.run(Command::ShaderTree(ShaderTreeEdit::SetNodeGraphDefault {
                            graph,
                            input,
                            value,
                            continuing: false,
                        }))
                    }
                    Err(_) => self.status = usage,
                }
            }
            _ => self.status = usage,
        }
        true
    }

    /// Run an image command line, or its help topic. False when `text`
    /// isn't one.
    pub(super) fn image_canonical(&mut self, text: &str) -> bool {
        if self.node_canonical(text) {
            return true;
        }
        let Ok(args) = crate::command::tokens(text) else {
            return false;
        };
        let Some(first) = args.first().map(|a| a.to_ascii_lowercase()) else {
            return false;
        };
        let help = |name: &str| {
            IMAGE_COMMANDS
                .iter()
                .find(|(c, _)| c.eq_ignore_ascii_case(name))
                .map(|(_, h)| *h)
        };
        if first == "help" && args.len() == 2 {
            if let Some(h) = help(&args[1].to_ascii_lowercase()) {
                self.status = h.into();
                return true;
            }
            return false;
        }
        let Some(usage) = help(&first) else {
            return false;
        };
        let usage = usage.trim_start_matches("Help: ").to_owned();
        let images = &self.editor.scene.images;
        match (first.as_str(), args.len()) {
            ("image.list", 1) => {
                self.status = if images.is_empty() {
                    "No images in the scene".into()
                } else {
                    images
                        .iter()
                        .map(|i| format!("{} — {} ({}×{})", i.name, i.path, i.width, i.height))
                        .collect::<Vec<_>>()
                        .join("; ")
                };
            }
            ("clip.addstill", 2) => {
                let path = args[1]
                    .split_once(':')
                    .filter(|(key, _)| key.eq_ignore_ascii_case("filename"))
                    .map_or(args[1].as_str(), |(_, path)| path);
                let path = std::path::PathBuf::from(path);
                self.load_image_file(&path);
            }
            ("clip.reload", 1 | 2) => {
                let all = "all".to_owned();
                let want = args.get(1).unwrap_or(&all);
                let paths: Vec<String> = images
                    .iter()
                    .filter(|i| {
                        want.eq_ignore_ascii_case("all") || i.name == *want || i.path == *want
                    })
                    .map(|i| i.path.clone())
                    .collect();
                if paths.is_empty() {
                    self.status = format!("No scene image named {want}");
                } else {
                    for path in &paths {
                        self.resident_cache.reload_image(path);
                        self.sky_cache.forget(path);
                    }
                    self.status = format!("Reloaded {}", paths.join(", "));
                }
            }
            _ => self.status = usage,
        }
        true
    }
}

/// The map's own command for a one-field change it names (UIC-165): a
/// Group's Polygon Tag / Tag Type, a texture's Locator or Vertex Map.
fn map_layer_line(old: &Node, new: &Node) -> Option<String> {
    use operandi_core::shader_tree::{NodeKind, TagMask, TextureLayer, TextureSource};
    let id = new.id;
    let brace = |t: &str| format!("{{{t}}}");
    match (&old.kind, &new.kind) {
        (NodeKind::Group(a), NodeKind::Group(b)) => {
            let rest_same =
                |x: &operandi_core::shader_tree::GroupMask,
                 y: &operandi_core::shader_tree::GroupMask| {
                    let mut x = x.clone();
                    x.tag = y.tag.clone();
                    x.tag_type = y.tag_type;
                    x == *y
                };
            if !rest_same(a, b) {
                return None;
            }
            if a.tag_type != b.tag_type && b.tag == TagMask::None {
                Some(format!(
                    "mask.setPTagType name:{} layer:{id}",
                    brace(b.tag_type.label())
                ))
            } else if a.tag_type == b.tag_type && a.tag != b.tag {
                let name = match &b.tag {
                    TagMask::All => "(all)".to_owned(),
                    TagMask::None => "(none)".to_owned(),
                    TagMask::Name(n) => n.clone(),
                };
                Some(format!("mask.setPTag name:{} layer:{id}", brace(&name)))
            } else {
                None
            }
        }
        (NodeKind::Texture(a), NodeKind::Texture(b)) => {
            let mut x = a.clone();
            x.locator = b.locator;
            if x == *b && a.locator != b.locator {
                let to = b.locator.map_or("none".to_owned(), |l| l.to_string());
                return Some(format!("texture.setLocator locator:{to} layer:{id}"));
            }
            if let (
                TextureLayer {
                    source: TextureSource::VertexMap { map: m1, zones: z1 },
                    ..
                },
                TextureLayer {
                    source: TextureSource::VertexMap { map: m2, zones: z2 },
                    ..
                },
            ) = (a, b)
                && z1 == z2
                && m1 != m2
            {
                let mut x = a.clone();
                x.source = b.source.clone();
                if x == *b {
                    let name = m2.as_ref().map_or("none".to_owned(), |m| m.name.clone());
                    return Some(format!("texture.setVMap name:{} layer:{id}", brace(&name)));
                }
            }
            None
        }
        _ => None,
    }
}

/// The command line for a Render Output edit that changes one setting.
pub(crate) fn output_edit_line<T: serde::Serialize>(
    id: u64,
    before: &T,
    after: &T,
) -> Option<String> {
    use operandi_core::channels::json_leaves;
    let (mut was, mut now) = (Vec::new(), Vec::new());
    json_leaves("", &serde_json::to_value(before).ok()?, &mut was);
    json_leaves("", &serde_json::to_value(after).ok()?, &mut now);
    if was.len() != now.len() {
        return None;
    }
    let changed: Vec<_> = was.iter().zip(&now).filter(|(a, b)| a != b).collect();
    let [(_, (path, leaf))] = changed.as_slice() else {
        return None;
    };
    let text = match leaf {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(parts) => parts
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(","),
        other => other.to_string(),
    };
    let text = if text.contains(' ') || text.is_empty() {
        format!("{{{text}}}")
    } else {
        text
    };
    Some(format!("item.channel {path} {text} layer:{id}"))
}

#[cfg(test)]
mod channel_light_tests {
    use super::*;
    use operandi_core::shader_tree::NewLayer;

    /// Closing a scene with a Shader Tree layer selected, then Add Layer
    /// in the next scene: the new layer goes in at the top level instead
    /// of under itself (the stale id is the next id), and the tree stays
    /// valid.
    #[test]
    fn add_layer_after_closing_a_scene_with_a_selection() {
        let mut app = Operandi::empty();
        for _ in 0..2 {
            let node = added(&mut app, NewLayer::ImageMap);
            app.shading.selected = vec![node.id];
        }
        let stale = app.shading.selected.clone();
        app.close_active_scene();
        assert_eq!(app.shading.selected, stale, "the selection lingers");
        app.shading_add(NewLayer::ImageMap);
        let tree = &app.editor.scene.shader_tree;
        tree.validate().unwrap();
        assert!(
            tree.nodes.iter().all(|n| n.parent != Some(n.id)),
            "{:?}",
            app.status
        );
    }

    fn added(app: &mut Operandi, layer: NewLayer) -> Node {
        app.run(Command::ShaderTree(ShaderTreeEdit::Add {
            layer,
            selected: None,
        }));
        let tree = &app.editor.scene.shader_tree;
        tree.nodes
            .iter()
            .max_by_key(|n| n.id)
            .cloned()
            .expect("the new layer")
    }

    fn lights(node: &Node, base: bool) -> Vec<(&'static str, bool)> {
        let mut fresh = node.clone();
        fresh.kind = node.kind.defaults(base).expect("defaults");
        let defaults = channel_defaults(fresh.channels());
        let mut node = node.clone();
        node.channels()
            .into_iter()
            .map(|(name, c)| {
                let d = defaults.iter().find(|(n, _)| *n == name).unwrap();
                (name, channel_value(&c) != d.1)
            })
            .collect()
    }

    /// CH-Q1: a new layer shows every light at the default; a changed
    /// channel lights; Reset to Default puts the default back, one step.
    #[test]
    fn new_layers_start_at_default_and_reset_restores() {
        let mut app = Operandi::empty();
        for kind in [NewLayer::Material, NewLayer::Constant, NewLayer::ImageMap] {
            let node = added(&mut app, kind);
            assert!(
                lights(&node, false).iter().all(|(_, changed)| !changed),
                "{kind:?}: {:?}",
                lights(&node, false)
            );
        }
        let mut node = added(&mut app, NewLayer::Material);
        for (name, c) in node.channels() {
            if name == "Diffuse Color" {
                set_channel(c, &ChannelValue::Vector([1., 0., 0.]));
            }
        }
        let changed: Vec<_> = lights(&node, false)
            .into_iter()
            .filter(|(_, c)| *c)
            .map(|(n, _)| n)
            .collect();
        assert_eq!(changed, ["Diffuse Color"]);
        app.run(Command::ShaderTree(ShaderTreeEdit::Replace {
            node: node.clone(),
            continuing: false,
        }));
        let steps = app.editor.history.undo.len();
        let mut fresh = node.clone();
        fresh.kind = node.kind.defaults(false).unwrap();
        let defaults = channel_defaults(fresh.channels());
        for (name, c) in node.channels() {
            if name == "Diffuse Color" {
                let d = &defaults.iter().find(|(n, _)| *n == name).unwrap().1;
                set_channel(c, d);
            }
        }
        app.run(Command::ShaderTree(ShaderTreeEdit::Replace {
            node: node.clone(),
            continuing: false,
        }));
        assert_eq!(app.editor.history.undo.len(), steps + 1);
        assert!(lights(&node, false).iter().all(|(_, c)| !c));
    }
}
