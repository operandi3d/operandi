//! The panel-type registry and panel groups (UI conventions bible
//! UIC-155..157).
//!
//! Every panel kind registers itself once in [`REGISTRY`]: its saved key,
//! its name, its category and how to draw it. The "+" menu after a group's
//! tabs, the layout's save and load, and any later "new window" command all
//! read this registry, so a new panel is added by registering it here, with
//! no menu edits. Categories with nothing registered are hidden.
//!
//! A [`PanelGroup`] is one slot of the side panel: several panels stacked
//! as tabs. Panel state (selection, search, scroll) belongs to the panel
//! kind, so each kind is open at most once in the layout (provisional): the
//! "+" menu greys a kind that is already open.
use super::*;
use crate::unavailable::Gate;
use serde::{Deserialize, Serialize};

/// The kinds of panel that can sit in a panel group.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelKind {
    Items,
    ShaderTree,
    Groups,
    Properties,
    /// Saved as `mesh_ops`, its name before 2026-11-23, so layouts open in
    /// older and newer builds alike.
    #[serde(rename = "mesh_ops")]
    MeshStack,
    /// The preset browser (PB-100): also has its own OS window.
    PresetBrowser,
    /// The active mesh's UV maps (UVE-1 section 1, uv_edit_ui.rs).
    UvMaps,
    /// The Command History (UIC-228, command_history_ui.rs).
    CommandHistory,
    /// A material's node view (node_material_ui.rs): docks like any panel
    /// and follows the Shader Tree's selected material.
    NodeView,
    /// Vertex Normal Tools (vertex_normal_ui.rs, G9 VN-0).
    VertexNormalTools,
    /// The Statistics panel (statistics_ui.rs): counts by group, +/− to
    /// select them.
    Statistics,
    /// A tab whose kind a layout file names but this build doesn't have
    /// (LAY-49, UIC-209): an empty panel showing the name its group keeps
    /// at this index (`PanelGroup::missing`). Never offered by "+".
    Missing(u8),
}

/// The "+" menu's categories, in menu order (UIC-156).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Views3d,
    Toolbars,
    Properties,
    DataLists,
    Info,
    Editors,
    Utilities,
    Application,
}

impl Category {
    pub const ALL: [Category; 8] = [
        Category::Views3d,
        Category::Toolbars,
        Category::Properties,
        Category::DataLists,
        Category::Info,
        Category::Editors,
        Category::Utilities,
        Category::Application,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Category::Views3d => "3D Views and Editing",
            Category::Toolbars => "Toolbars",
            Category::Properties => "Properties",
            Category::DataLists => "Data Lists",
            Category::Info => "Info",
            Category::Editors => "Editors",
            Category::Utilities => "Utilities",
            Category::Application => "Application",
        }
    }
}

/// One registered panel type (UIC-157).
pub struct PanelType {
    pub kind: PanelKind,
    /// The name on its tab and in the "+" menu.
    pub name: &'static str,
    pub category: Category,
    /// Draws the panel's body into the rect under the group's tabs.
    pub draw: fn(&mut Operandi, &mut Ui, Rect),
}

/// Every panel kind Operandi has.
pub static REGISTRY: &[PanelType] = &[
    PanelType {
        kind: PanelKind::Items,
        name: "Items",
        category: Category::DataLists,
        draw: |a, ui, rect| a.items_body(ui, rect),
    },
    PanelType {
        kind: PanelKind::ShaderTree,
        name: "Shader Tree",
        category: Category::DataLists,
        draw: |a, ui, rect| a.shading_section(ui, rect),
    },
    PanelType {
        kind: PanelKind::Groups,
        name: "Groups",
        category: Category::DataLists,
        draw: |a, ui, rect| a.groups_section(ui, rect),
    },
    PanelType {
        kind: PanelKind::Properties,
        name: "Properties",
        category: Category::Properties,
        draw: |a, ui, rect| a.properties_body(ui, rect, "properties-category"),
    },
    PanelType {
        kind: PanelKind::MeshStack,
        name: "Mesh Stack",
        category: Category::DataLists,
        draw: |a, ui, rect| a.mesh_stack_section(ui, rect),
    },
    PanelType {
        kind: PanelKind::UvMaps,
        name: "UV Maps",
        category: Category::DataLists,
        draw: |a, ui, rect| a.uv_maps_section(ui, rect),
    },
    PanelType {
        kind: PanelKind::PresetBrowser,
        name: "Preset Browser",
        category: Category::Utilities,
        draw: |a, ui, _| a.preset_browser_body(ui, false),
    },
    PanelType {
        kind: PanelKind::CommandHistory,
        name: "Command History",
        category: Category::Utilities,
        draw: |a, ui, rect| a.command_history_panel(ui, rect),
    },
    PanelType {
        kind: PanelKind::VertexNormalTools,
        name: "Vertex Normal Tools",
        category: Category::Utilities,
        draw: |a, ui, _| a.vertex_normal_panel(ui),
    },
    PanelType {
        kind: PanelKind::Statistics,
        name: "Statistics",
        category: Category::Info,
        draw: |a, ui, rect| a.statistics_panel(ui, rect),
    },
    PanelType {
        kind: PanelKind::NodeView,
        name: "Node View",
        category: Category::Editors,
        draw: |a, ui, rect| a.node_view_body(ui, rect),
    },
];

/// What a [`PanelKind::Missing`] tab draws: nothing (the group shows its
/// name, workspace.rs).
static MISSING: PanelType = PanelType {
    kind: PanelKind::Missing(0),
    name: "Missing",
    category: Category::Utilities,
    draw: |_, _, _| {},
};

pub fn panel_type(kind: PanelKind) -> &'static PanelType {
    if let PanelKind::Missing(_) = kind {
        return &MISSING;
    }
    REGISTRY
        .iter()
        .find(|t| t.kind == kind)
        .expect("every panel kind is registered")
}

/// The "+" menu's categories with their registered panels; empty
/// categories are left out.
pub fn menu() -> Vec<(Category, Vec<&'static PanelType>)> {
    Category::ALL
        .iter()
        .map(|&c| (c, REGISTRY.iter().filter(|t| t.category == c).collect()))
        .filter(|(_, types): &(Category, Vec<_>)| !types.is_empty())
        .collect()
}

/// One slot's tabs: which panels, in what order, and which is in front.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelGroup {
    #[serde(skip)]
    pub id: &'static str,
    pub tabs: Vec<PanelKind>,
    pub active: usize,
    /// Each tab's rect, and the "+" button's, from the last frame (tests).
    #[serde(skip)]
    pub rects: Vec<Rect>,
    #[serde(skip, default = "nothing")]
    pub plus_rect: Rect,
    /// The names of this group's [`PanelKind::Missing`] tabs.
    #[serde(skip)]
    pub missing: Vec<String>,
}

impl PanelGroup {
    pub fn new(id: &'static str, tabs: &[PanelKind]) -> Self {
        Self {
            id,
            tabs: tabs.to_vec(),
            active: 0,
            rects: Vec::new(),
            plus_rect: Rect::NOTHING,
            missing: Vec::new(),
        }
    }
    /// A tab's name: the registry's, or the one a layout file gave a kind
    /// this build doesn't have.
    pub fn name(&self, kind: PanelKind) -> String {
        match kind {
            PanelKind::Missing(i) => self
                .missing
                .get(usize::from(i))
                .cloned()
                .unwrap_or_else(|| "Missing".into()),
            k => panel_type(k).name.into(),
        }
    }
    /// The panel in front, if the group has any.
    pub fn front(&self) -> Option<PanelKind> {
        self.tabs.get(self.active).copied()
    }
    /// Bring a tab to the front (as if the user clicked it; the shader
    /// layer menu's Properties entry asks for it, UIC-156).
    pub fn show_page(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active = index;
        }
    }
    pub fn contains(&self, kind: PanelKind) -> bool {
        self.tabs.contains(&kind)
    }
    /// Add a panel at the end and bring it to the front: the user asked
    /// for it, so this doesn't break UIC-21 (UIC-156).
    pub fn add(&mut self, kind: PanelKind) {
        self.tabs.retain(|&k| k != kind);
        self.tabs.push(kind);
        self.active = self.tabs.len() - 1;
    }
    /// Take a panel out; the front tab stays in front where it can.
    pub fn remove(&mut self, kind: PanelKind) {
        let front = self.front();
        self.tabs.retain(|&k| k != kind);
        self.active = front
            .and_then(|f| self.tabs.iter().position(|&k| k == f))
            .unwrap_or(0)
            .min(self.tabs.len().saturating_sub(1));
    }
    /// Move the tab at `from` so it lands before index `to` (an insert
    /// position, 0..=len). The front tab stays the same panel.
    pub fn reorder(&mut self, from: usize, to: usize) {
        if from >= self.tabs.len() {
            return;
        }
        let front = self.front();
        let kind = self.tabs.remove(from);
        let to = if to > from { to - 1 } else { to }.min(self.tabs.len());
        self.tabs.insert(to, kind);
        if let Some(f) = front {
            self.active = self.tabs.iter().position(|&k| k == f).unwrap_or(0);
        }
    }
    /// Move unique source tabs into this row. A duplicate in front selects
    /// the existing destination tab in its original position (D-02, D-04).
    pub fn merge_from(&mut self, source: &mut Self, at: usize) {
        let mut at = at.min(self.tabs.len());
        let mut front = self.front();
        for (index, &kind) in source.tabs.iter().enumerate() {
            let kind = match kind {
                PanelKind::Missing(_) => {
                    let name = source.name(kind);
                    if let Some(existing) = self.missing.iter().position(|other| *other == name) {
                        PanelKind::Missing(existing as u8)
                    } else {
                        let index = self.missing.len();
                        self.missing.push(name);
                        PanelKind::Missing(index as u8)
                    }
                }
                known => known,
            };
            if !self.contains(kind) {
                self.tabs.insert(at, kind);
                at += 1;
            }
            if index == source.active {
                front = Some(kind);
            }
        }
        if let Some(active) = front.and_then(|kind| self.tabs.iter().position(|&k| k == kind)) {
            self.active = active;
        }
        *source = Self::new(source.id, &[]);
    }
    /// Take saved tabs, keeping only known, unique kinds; nothing usable
    /// keeps the current tabs.
    pub fn restore(&mut self, saved: &PanelGroup) {
        let mut tabs = Vec::new();
        for &k in &saved.tabs {
            if !tabs.contains(&k) {
                tabs.push(k);
            }
        }
        if tabs.is_empty() {
            return;
        }
        self.tabs = tabs;
        self.active = saved.active.min(self.tabs.len() - 1);
    }
}

/// Tab width bounds (UIC-155): names are cut off with "…" below the
/// widest, and tabs that still don't fit go to the ▾ list.
const TAB_MAX: f32 = 96.;
const TAB_MIN: f32 = 44.;
const PLUS: f32 = 18.;
const OVERFLOW: f32 = 16.;

/// How many tabs fit in `width`, and each one's width.
pub fn tab_layout(names: &[f32], width: f32) -> (usize, f32) {
    let n = names.len();
    if n == 0 {
        return (0, TAB_MAX);
    }
    let room = (width - PLUS - 2.).max(0.);
    let each = (room / n as f32).min(TAB_MAX);
    if each >= TAB_MIN {
        return (n, each);
    }
    let room = (room - OVERFLOW).max(0.);
    let fit = ((room / TAB_MIN).floor() as usize).clamp(1, n);
    (fit, (room / fit as f32).min(TAB_MAX))
}

fn nothing() -> Rect {
    Rect::NOTHING
}

/// What the tab strip asks for this frame.
pub enum TabAction {
    None,
    Add(PanelKind),
    Close(PanelKind),
}

impl Operandi {
    /// Draws a group's tab strip (UIC-155/156): tabs with cut-off names,
    /// a ▾ for tabs that don't fit, then "+". Returns each shown tab's
    /// response (the caller adds its right-click menu: Float, Close Tab)
    /// and what the user asked for.
    pub(super) fn group_tabs(
        group: &mut PanelGroup,
        ui: &mut Ui,
        open_elsewhere: &[PanelKind],
        detached: &[PanelKind],
    ) -> (Vec<(PanelKind, Response)>, TabAction) {
        use theme::workspace as palette;
        let mut responses = Vec::new();
        let mut action = TabAction::None;
        let (strip, _) = ui.allocate_exact_size(vec2(ui.available_width(), 22.), Sense::hover());
        ui.painter().rect_filled(strip, 0., palette::tab_strip());
        let left = strip.left();
        let font = crate::typography::standard_font();
        let names: Vec<f32> = group
            .tabs
            .iter()
            .map(|&k| {
                ui.fonts_mut(|f| {
                    f.layout_no_wrap(group.name(k), font.clone(), Color32::WHITE)
                        .size()
                        .x
                })
            })
            .collect();
        let (fit, width) = tab_layout(&names, strip.right() - left);
        // Keep the front tab visible: when it would overflow, it takes the
        // last visible place.
        let mut shown: Vec<usize> = (0..fit).collect();
        if group.active >= fit && fit > 0 {
            shown[fit - 1] = group.active;
        }
        group.rects = vec![Rect::NOTHING; group.tabs.len()];
        for (slot, &index) in shown.iter().enumerate() {
            let kind = group.tabs[index];
            let name = group.name(kind);
            let tab = Rect::from_min_size(
                pos2(left + slot as f32 * width, strip.top()),
                vec2(width, 22.),
            );
            group.rects[index] = tab;
            let response = ui.interact(tab, Id::new((group.id, kind)), Sense::click_and_drag());
            let active = group.active == index;
            response.widget_info(|| {
                WidgetInfo::selected(WidgetType::SelectableLabel, true, active, &name)
            });
            theme::chrome::own_activation(ui, &response);
            if response.clicked() {
                group.active = index;
            }
            ui.painter().rect_filled(
                tab,
                CornerRadius {
                    nw: 3,
                    ne: 3,
                    sw: 0,
                    se: 0,
                },
                if active {
                    palette::tab()
                } else {
                    palette::tab_strip()
                },
            );
            if active && group.tabs.len() > 1 {
                ui.painter().rect_filled(
                    Rect::from_min_max(tab.min, pos2(tab.right(), tab.top() + 2.)),
                    CornerRadius {
                        nw: 3,
                        ne: 3,
                        ..CornerRadius::ZERO
                    },
                    theme::chrome::active(),
                );
            }
            // Names that don't fit are cut off with "…" (UIC-155).
            let mut job = egui::text::LayoutJob::single_section(
                name.clone(),
                egui::TextFormat {
                    font_id: font.clone(),
                    // Active tab name in accent, the rest dimmer.
                    color: if active {
                        theme::orange()
                    } else {
                        palette::tab_text()
                    },
                    ..Default::default()
                },
            );
            job.wrap = egui::text::TextWrapping {
                max_width: (width - 10.).max(8.),
                max_rows: 1,
                break_anywhere: true,
                overflow_character: Some('…'),
            };
            let galley = ui.painter().layout_job(job);
            let full = galley.size().x < names[index] - 0.5;
            ui.painter()
                .galley(tab.center() - galley.size() / 2., galley, theme::text());
            let response = if full {
                response.on_hover_text(&name)
            } else {
                response
            };
            responses.push((kind, response));
        }
        let mut x = left + shown.len() as f32 * width;
        // ▾: the tabs that didn't fit.
        if shown.len() < group.tabs.len() {
            let rect = Rect::from_min_size(pos2(x, strip.top()), vec2(OVERFLOW, 22.));
            let hidden: Vec<usize> = (0..group.tabs.len())
                .filter(|i| !shown.contains(i))
                .collect();
            let button = ui.put(rect, Button::new("▾").frame(false));
            egui::Popup::menu(&button).show(|ui| {
                for i in hidden {
                    if ui.button(group.name(group.tabs[i])).clicked() {
                        group.active = i;
                        ui.close();
                    }
                }
            });
            x += OVERFLOW;
        }
        // +: add a panel from the registry (UIC-156).
        let plus = Rect::from_min_size(pos2(x + 2., strip.top()), vec2(PLUS, 22.));
        group.plus_rect = plus;
        let button = ui
            .put(plus, Button::new("+").frame(false))
            .on_hover_text("Add a panel to this group");
        egui::Popup::menu(&button).show(|ui| {
            for (category, types) in menu() {
                ui.menu_button(category.label(), |ui| {
                    for t in types {
                        let here = group.contains(t.kind);
                        let elsewhere = open_elsewhere.contains(&t.kind);
                        let response = ui.gated(
                            if detached.contains(&t.kind) {
                                Some("Already in its own panel")
                            } else {
                                crate::unavailable::unless(!here, "Already in this group")
                            },
                            Button::new(t.name),
                        );
                        let response = if elsewhere {
                            response.on_hover_text("Open in another group: it moves here")
                        } else {
                            response
                        };
                        if response.clicked() {
                            action = TabAction::Add(t.kind);
                            ui.close();
                        }
                    }
                });
            }
        });
        (responses, action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Saved layouts name the Mesh Stack panel `mesh_ops`, its name before
    /// 2026-11-23, so layouts open in older and newer builds.
    #[test]
    fn mesh_stack_keeps_its_saved_name() {
        let text = serde_json::to_string(&PanelKind::MeshStack).unwrap();
        assert_eq!(text, "\"mesh_ops\"");
        let back: PanelKind = serde_json::from_str(&text).unwrap();
        assert_eq!(back, PanelKind::MeshStack);
    }

    #[test]
    fn every_kind_is_registered_once_and_empty_categories_are_hidden() {
        for kind in [
            PanelKind::Items,
            PanelKind::ShaderTree,
            PanelKind::Groups,
            PanelKind::Properties,
            PanelKind::MeshStack,
            PanelKind::PresetBrowser,
            PanelKind::UvMaps,
            PanelKind::CommandHistory,
            PanelKind::NodeView,
            PanelKind::VertexNormalTools,
            PanelKind::Statistics,
        ] {
            assert_eq!(REGISTRY.iter().filter(|t| t.kind == kind).count(), 1);
        }
        let shown: Vec<_> = menu().into_iter().map(|(c, _)| c).collect();
        assert_eq!(
            shown,
            [
                Category::Properties,
                Category::DataLists,
                Category::Info,
                Category::Editors,
                Category::Utilities
            ]
        );
        let lists: Vec<_> = menu()[1].1.iter().map(|t| t.name).collect();
        assert_eq!(
            lists,
            ["Items", "Shader Tree", "Groups", "Mesh Stack", "UV Maps"]
        );
    }

    #[test]
    fn adding_brings_the_new_tab_to_the_front_at_the_end() {
        let mut g = PanelGroup::new("g", &[PanelKind::Items, PanelKind::Groups]);
        g.add(PanelKind::ShaderTree);
        assert_eq!(
            g.tabs,
            [PanelKind::Items, PanelKind::Groups, PanelKind::ShaderTree]
        );
        assert_eq!(g.front(), Some(PanelKind::ShaderTree));
        g.show_page(0);
        g.remove(PanelKind::Groups);
        assert_eq!(g.front(), Some(PanelKind::Items), "front tab stays");
        g.remove(PanelKind::Items);
        assert_eq!(g.front(), Some(PanelKind::ShaderTree));
    }

    #[test]
    fn reorder_keeps_the_front_panel_in_front() {
        let mut g = PanelGroup::new(
            "g",
            &[PanelKind::Items, PanelKind::ShaderTree, PanelKind::Groups],
        );
        g.show_page(1);
        g.reorder(0, 3);
        assert_eq!(
            g.tabs,
            [PanelKind::ShaderTree, PanelKind::Groups, PanelKind::Items]
        );
        assert_eq!(g.front(), Some(PanelKind::ShaderTree));
        g.reorder(2, 0);
        assert_eq!(g.tabs[0], PanelKind::Items);
        assert_eq!(g.front(), Some(PanelKind::ShaderTree));
    }

    #[test]
    fn saved_tabs_round_trip_and_bad_ones_are_ignored() {
        let mut g = PanelGroup::new("g", &[PanelKind::Items, PanelKind::ShaderTree]);
        g.add(PanelKind::Groups);
        let text = serde_json::to_string(&g).unwrap();
        assert!(text.contains("shader_tree"));
        let back: PanelGroup = serde_json::from_str(&text).unwrap();
        let mut fresh = PanelGroup::new("g", &[PanelKind::Items]);
        fresh.restore(&back);
        assert_eq!(fresh.tabs, g.tabs);
        assert_eq!(fresh.active, 2);
        let empty: PanelGroup = serde_json::from_str(r#"{"tabs":[],"active":4}"#).unwrap();
        fresh.restore(&empty);
        assert_eq!(fresh.tabs, g.tabs, "nothing usable keeps the tabs");
        assert!(
            serde_json::from_str::<PanelGroup>(r#"{"tabs":["no_such_panel"],"active":0}"#).is_err()
        );
    }

    #[test]
    fn tabs_shrink_then_overflow_into_the_list() {
        let names = [40., 70., 50.];
        assert_eq!(tab_layout(&names, 400.), (3, TAB_MAX));
        let (fit, w) = tab_layout(&names, 3. * 60. + PLUS + 2.);
        assert_eq!(fit, 3);
        assert!((TAB_MIN..TAB_MAX).contains(&w));
        let (fit, w) = tab_layout(&names, 2. * TAB_MIN + PLUS + OVERFLOW + 3.);
        assert_eq!(fit, 2);
        assert!(w >= TAB_MIN);
        assert_eq!(tab_layout(&names, 10.).0, 1, "the front tab always shows");
    }
}
