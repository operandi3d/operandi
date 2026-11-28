//! Scoped workspace controls. UI layout/configuration never owns modeling data.
use super::*;
use crate::prefs_file::PrefsFile;
use crate::unavailable::Gate;
use theme::workspace as palette;

/// GL-13: a Group Locator's own Properties tab.
pub(super) const GROUP_LOCATOR_TAB: &str = "Group Locator";
use operandi_core::locator::PARENT_TO_GROUP_LOCATOR;
pub(super) fn number_text(value: f64, unit: &str) -> String {
    // Distances show in the user's Distance Units.
    if unit == " m" {
        return operandi_core::units::format_distance(value);
    }
    if value.abs() < 1e-12 {
        return format!("0{unit}");
    }
    if value != 0. && value.abs() < 0.00001 {
        let text = format!("{value:.12}");
        return format!("{}{unit}", text.trim_end_matches('0').trim_end_matches('.'));
    }
    let text = format!("{value:.5}");
    format!("{}{unit}", text.trim_end_matches('0').trim_end_matches('.'))
}

impl Operandi {
    /// A component-mode button. Its width comes from the base label so a
    /// held modifier never reflows the bar; label, lit state and shortcut
    /// hint come from whatever the button resolves to (button_variants.rs).
    pub(super) fn component_button(
        &mut self,
        ui: &mut Ui,
        button: button_variants::ButtonId,
    ) -> Response {
        // Wide enough for the base and every variant, so a held key never
        // reflows the bar (MB: the button keeps its width).
        let font = crate::typography::standard_font();
        let label_text = |this: &Self, shown: &button_variants::Shown| {
            format!(
                "{}{}",
                shown.label,
                this.shortcut(shown.action.shortcut_action().unwrap_or_default())
            )
        };
        let text_width = std::iter::once(button_variants::Mods::NONE)
            .chain(button_variants::variants(button).iter().map(|v| v.mods))
            .map(|mods| {
                let text = label_text(self, &button_variants::resolve(button, mods));
                ui.painter()
                    .layout_no_wrap(text, font.clone(), theme::text())
                    .size()
                    .x
            })
            .fold(0., f32::max);
        let (rect, response) = ui.allocate_exact_size(vec2(text_width + 30., 22.), Sense::click());
        let shown = self.shown(button);
        let text = label_text(self, &shown);
        let active = self.action_active(shown.action);
        let enabled = self.action_enabled(shown.action);
        ui.painter()
            .rect_filled(rect, 2., theme::chrome::button_fill(&response, active));
        let mode = match button.base() {
            button_variants::ButtonAction::Mode(mode) => mode,
            _ => Mode::Polygon,
        };
        icons::paint(
            ui,
            Rect::from_min_size(rect.min + vec2(2., 1.), vec2(20., 20.)),
            match mode {
                Mode::Vertex => icons::Icon::Vertex,
                Mode::Edge => icons::Icon::Edge,
                _ => icons::Icon::Polygon,
            },
        );
        ui.painter().text(
            rect.left_center() + vec2(25., 0.),
            Align2::LEFT_CENTER,
            &text,
            font,
            if !enabled {
                theme::disabled_text()
            } else if active {
                theme::chrome::active_text()
            } else {
                theme::text()
            },
        );
        response.widget_info(|| {
            WidgetInfo::selected(WidgetType::SelectableLabel, enabled, active, &text)
        });
        theme::chrome::own_activation(ui, &response);
        theme::chrome::focus(ui, rect, &response);
        let tooltip = self.variant_tooltip(button);
        let why = self.action_unavailable(shown.action);
        crate::unavailable::explain(response, why).on_hover_text(tooltip)
    }
    pub(super) fn right_sidebar(&mut self, ctx: &Context) {
        let items = !self.is_floating(floating::Panel::Items);
        let properties = !self.is_floating(floating::Panel::Properties);
        if !items && !properties {
            // Both float: the command line keeps a strip of its own.
            self.command_history_strip(ctx);
            return;
        }
        if self.layouts.right_collapsed {
            self.collapsed_side(ctx, collapse_line::Bar::Right);
            return;
        }
        // Drawn after the toolbox, so the available width already leaves it out.
        let max_right = layout::right_max(ctx.available_rect().width(), 0.);
        // MN-08: a maximised Items or Properties group fills the column.
        use operandi_layout::Pane;
        let (items, properties) = match self.panel_maximized {
            Some(Pane::Items) if items && properties => (true, false),
            Some(Pane::Properties) if items && properties => (false, true),
            _ => (items, properties),
        };
        self.side_panel_rects.remove(&Pane::Items);
        self.side_panel_rects.remove(&Pane::Properties);
        SidePanel::right("scene")
            .resizable(false)
            .default_width(layout::RIGHT_DEFAULT)
            .width_range(layout::RIGHT_MIN..=max_right)
            .frame(Frame::side_top_panel(&ctx.style()).inner_margin(4.))
            .show(ctx, |ui| {
                // The Command History dock: the entry line, plus its tabs
                // when dragged open (HIS-01).
                let reserved = self.command_history_reserved();
                let total = (ui.available_height() - reserved).max(200.);
                if !(items && properties) {
                    let (rect, _) =
                        ui.allocate_exact_size(vec2(ui.available_width(), total), Sense::hover());
                    if items {
                        self.side_panel_rects.insert(Pane::Items, rect);
                        self.items_section(ui, rect);
                    } else {
                        self.side_panel_rects.insert(Pane::Properties, rect);
                        self.properties_section(ui, rect);
                    }
                    self.command_history_dock(ui);
                    return;
                }
                let height = (total * self.scene_fraction).clamp(80., total - 80.);
                let (upper, _) =
                    ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
                self.side_panel_rects.insert(Pane::Items, upper);
                self.items_section(ui, upper);
                let (divider, response) =
                    ui.allocate_exact_size(vec2(ui.available_width(), 4.), Sense::hover());
                self.scene_divider_rect = divider;
                self.scene_available_height = total;
                ui.painter().hline(
                    divider.x_range(),
                    divider.center().y,
                    Stroke::new(1.0f32, theme::border()),
                );
                if response.hovered() {
                    ctx.set_cursor_icon(CursorIcon::ResizeVertical);
                }
                let height = (ui.available_height() - reserved).max(40.);
                let (lower, _) =
                    ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
                self.side_panel_rects.insert(Pane::Properties, lower);
                self.properties_section(ui, lower);
                self.command_history_dock(ui);
            });
    }
    pub(super) fn command_history_strip(&mut self, ctx: &Context) {
        TopBottomPanel::bottom("command-entry")
            .frame(Frame::side_top_panel(&ctx.style()).inner_margin(4.))
            .show(ctx, |ui| self.command_history_dock(ui));
    }

    /// True while a group shows the Shader Tree in front (its Properties
    /// then show the selected layer).
    pub(super) fn shading_tab(&self) -> bool {
        self.front_panel(panel_registry::PanelKind::ShaderTree)
    }
    /// The Mesh Stack tab is in front with an op of the active mesh selected:
    /// Properties shows that op (MO-10, MO-19).
    pub(super) fn mesh_op_tab(&self) -> bool {
        self.front_panel(panel_registry::PanelKind::MeshStack) && self.selected_mesh_op().is_some()
    }
    /// Is `kind` the front tab of either panel group?
    pub(super) fn front_panel(&self, kind: panel_registry::PanelKind) -> bool {
        [
            (floating::Panel::Items, &self.upper_frame),
            (floating::Panel::Properties, &self.lower_frame),
        ]
        .into_iter()
        .any(|(panel, group)| {
            group.front() == Some(kind)
                && (!self.layouts.mixed_dock || self.dock_group_visible(panel))
        }) || self.dock_tab_visible(kind)
    }
    pub(super) fn focus_scene_list(&mut self, ui: &Ui, kind: panel_registry::PanelKind) {
        self.scene_focus_tab = Some(kind);
        ui.memory_mut(|m| m.request_focus(Id::new("scene-list-focus")));
    }
    /// The upper panel group (Items, Shader Tree, Groups by default).
    pub(super) fn items_section(&mut self, ui: &mut Ui, upper: Rect) {
        self.group_section(ui, upper, upper, false);
    }
    /// The lower panel group (Properties by default).
    pub(super) fn properties_section(&mut self, ui: &mut Ui, lower: Rect) {
        self.group_section(ui, lower, lower, true);
    }
    pub(super) fn sidebar_group(&self, panel: floating::Panel) -> &panel_registry::PanelGroup {
        match panel {
            floating::Panel::Items => &self.upper_frame,
            floating::Panel::Properties => &self.lower_frame,
            floating::Panel::Toolbox => unreachable!("Toolbox has no sidebar group"),
        }
    }

    pub(super) fn sidebar_group_mut(
        &mut self,
        panel: floating::Panel,
    ) -> &mut panel_registry::PanelGroup {
        match panel {
            floating::Panel::Items => &mut self.upper_frame,
            floating::Panel::Properties => &mut self.lower_frame,
            floating::Panel::Toolbox => unreachable!("Toolbox has no sidebar group"),
        }
    }

    /// One panel group: its tabs and "+" (UIC-155/156), its corner dot,
    /// then the front panel, drawn through the registry (UIC-157).
    pub(super) fn group_section(&mut self, ui: &mut Ui, area: Rect, dock_rect: Rect, lower: bool) {
        use panel_registry::{PanelKind, TabAction};
        let float = if lower {
            floating::Panel::Properties
        } else {
            floating::Panel::Items
        };
        // A child Ui the caller has already made room for: nothing drawn
        // inside can widen the side panel.
        let mut child = ui.new_child(UiBuilder::new().max_rect(area));
        self.panel_dot(ui, crate::panel_dot::Dot::Side(float), dock_rect, None);
        let ui = &mut child;
        {
            ui.set_clip_rect(area.intersect(ui.clip_rect()));
            // The header strip takes the right-click menu; the tabs sit on
            // top of it (registered later) so they still get clicks, and
            // carry the same menu plus Close Tab.
            let header = Rect::from_min_size(area.min, vec2(area.width(), 22.));
            let response = ui.interact(
                header,
                Id::new((float.label(), "group-header")),
                Sense::click(),
            );
            self.panel_header_menu(&response, float, None);
            let other: Vec<PanelKind> = if lower {
                self.upper_frame.tabs.clone()
            } else {
                self.lower_frame.tabs.clone()
            };
            let detached: Vec<_> = self
                .views
                .dock_tabs
                .values()
                .filter_map(|slot| layouts::kind_named(&slot.kind))
                .collect();
            let group = if lower {
                &mut self.lower_frame
            } else {
                &mut self.upper_frame
            };
            let (tabs, mut action) = Operandi::group_tabs(group, ui, &other, &detached);
            let many = group.tabs.len() > 1;
            self.dock_panel_tab_row(ui, float, header);
            for (kind, tab) in tabs {
                self.track_tab_drag(
                    &tab,
                    crate::panel_dot::TabSource::Panel(float, kind),
                    header,
                    area.size(),
                );
                if let Some(close) = self.panel_header_menu(&tab, float, many.then_some(kind)) {
                    action = TabAction::Close(close);
                }
            }
            match action {
                TabAction::None => {}
                TabAction::Add(kind) => {
                    // A kind lives in one group: adding it here moves it,
                    // unless it is the other group's only tab.
                    let (here, there) = if lower {
                        (&mut self.lower_frame, &mut self.upper_frame)
                    } else {
                        (&mut self.upper_frame, &mut self.lower_frame)
                    };
                    if !there.contains(kind) || there.tabs.len() > 1 {
                        there.remove(kind);
                        here.add(kind);
                    } else {
                        self.status = format!(
                            "{} is the other group's only tab",
                            panel_registry::panel_type(kind).name
                        );
                    }
                }
                TabAction::Close(kind) => {
                    let group = if lower {
                        &mut self.lower_frame
                    } else {
                        &mut self.upper_frame
                    };
                    if group.tabs.len() > 1 {
                        group.remove(kind);
                    }
                }
            }
            let front = if lower {
                self.lower_frame.front()
            } else {
                self.upper_frame.front()
            };
            let body = Rect::from_min_max(pos2(area.left(), ui.cursor().top()), area.max);
            if !lower || self.upper_frame.tabs.is_empty() {
                self.scene_rect = area;
            }
            if let Some(kind) = front {
                let name = self.sidebar_group(float).name(kind);
                self.registry_panel_body(ui, area, body, kind, &name);
            }
        }
    }

    pub(super) fn registry_panel_body(
        &mut self,
        ui: &mut Ui,
        area: Rect,
        body: Rect,
        kind: panel_registry::PanelKind,
        name: &str,
    ) {
        use panel_registry::PanelKind;
        self.help_region(area, crate::help_mode::panel_help_id(kind));
        // UIC-20 / D-12: only a drawn list may take scene-list focus.
        if kind != PanelKind::Properties && !matches!(kind, PanelKind::Missing(_)) {
            let first_list = self.visible_scene_lists.is_empty();
            self.visible_scene_lists.insert(kind, area);
            if first_list {
                ui.interact(
                    area,
                    Id::new("scene-list-focus"),
                    Sense::focusable_noninteractive(),
                );
            }
        }
        if matches!(kind, PanelKind::Missing(_)) {
            ui.painter().text(
                body.center(),
                Align2::CENTER_CENTER,
                format!("{name}\n(not in this version)"),
                crate::typography::standard_font(),
                theme::disabled_text(),
            );
        } else {
            let mut inner = ui.new_child(UiBuilder::new().max_rect(body));
            (panel_registry::panel_type(kind).draw)(self, &mut inner, body);
        }
    }

    /// Selected-item Properties: category tabs down the right edge, one per
    /// kind of property the selection has, and the rows beside them.
    /// `tabs_id` keeps two copies (the panel and Mini Properties) apart.
    pub(super) fn properties_body(&mut self, ui: &mut Ui, body: Rect, tabs_id: &str) {
        let tabs = self.property_tabs();
        let active = tabs
            .iter()
            .position(|t| *t == self.properties_tab)
            .unwrap_or(0);
        let tab = tabs.get(active).copied().unwrap_or("");
        let content = if tabs.is_empty() {
            body
        } else {
            let strip = Rect::from_min_max(
                pos2(body.right() - crate::side_tabs::WIDTH, body.top()),
                body.max,
            );
            if let Some(index) = crate::side_tabs::show(
                ui,
                tabs_id,
                strip,
                &tabs,
                active,
                &mut self.properties_tab_rects,
            ) {
                self.properties_tab = tabs[index].to_owned();
            }
            Rect::from_min_max(body.min, pos2(strip.left() - 2., body.bottom()))
        };
        ui.scope_builder(UiBuilder::new().max_rect(content), |ui| {
            ScrollArea::vertical()
                .id_salt("item-properties")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // Rows stop short of the scroll bar (it floats over
                    // the content otherwise).
                    let room = crate::form::layout::scroll_bar_room(ui);
                    ui.set_max_width((ui.available_width() - room).max(0.));
                    // One tab's rows share one label edge (UIC-111).
                    crate::form::rows::scope(ui, &format!("properties/{tab}"), |ui| {
                        if self.shading_tab() {
                            self.shading_properties(ui);
                        } else if self.mesh_op_tab() {
                            self.mesh_op_properties_panel(ui);
                        } else {
                            self.item_properties(ui, tab);
                        }
                    });
                });
        });
    }
    /// The Properties panel's categories for what is selected now.
    pub(super) fn property_tabs(&self) -> Vec<&'static str> {
        use operandi_core::shader_tree::NodeKind;
        if self.mesh_op_tab() {
            return vec![crate::mesh_stack_ui::MESH_OP_TAB];
        }
        if self.shading_tab() {
            use crate::environment_ui::{
                ENVIRONMENT_MATERIAL_TAB, ENVIRONMENT_TAB, EnvironmentRow,
            };
            match self.shading.environment {
                Some(EnvironmentRow::Item(_)) => return vec![ENVIRONMENT_TAB],
                Some(EnvironmentRow::Material(_)) => return vec![ENVIRONMENT_MATERIAL_TAB],
                Some(EnvironmentRow::Layer(..)) => {
                    return vec![crate::environment_ui::ENVIRONMENT_LAYER_TAB];
                }
                None => {}
            }
            if self.render_item_selected() {
                return crate::render_item_ui::TABS.to_vec();
            }
            let tree = &self.editor.scene.shader_tree;
            let Some(node) = self.shading.selected.last().and_then(|&id| tree.get(id)) else {
                return Vec::new();
            };
            // Material editing item 4: a Material's rows sit in tabs.
            if matches!(node.kind, NodeKind::Material(_)) {
                return crate::material_layout::TABS.to_vec();
            }
            return vec![match node.kind {
                NodeKind::RenderOutput(_) => "Render Output",
                NodeKind::Material(_) => crate::material_layout::MATERIAL_REF_TAB,
                NodeKind::Shader(_) => "Shader",
                NodeKind::Group(_) => "Group",
                NodeKind::Folder => "Folder",
                NodeKind::Texture(_) => "Texture",
                NodeKind::NodeMaterial(_) => "Node Material",
            }];
        }
        // Display and Assembly come with every item kind (shared tabs,
        // item_common_ui.rs); a light's own tab is named after its kind
        // (LP-01).
        use crate::item_common_ui::{ASSEMBLY_TAB, DISPLAY_TAB};
        use crate::item_extras_ui::{CHANNELS_TAB, TAGS_TAB};
        let shared = [DISPLAY_TAB, ASSEMBLY_TAB, CHANNELS_TAB, TAGS_TAB];
        let mut tabs = match self.editor.scene.active() {
            None => return Vec::new(),
            Some(i) if i.backdrop.is_some() => vec!["Backdrop"],
            // CP-01: Camera View and Camera Effects; Camera Overlay while
            // Display > Show Camera Overlay is on (CP-D12).
            Some(i) if i.camera.is_some() => {
                use crate::camera_props_ui::{EFFECTS_TAB, OVERLAY_TAB, VIEW_TAB};
                let mut tabs = vec![VIEW_TAB, EFFECTS_TAB];
                if i.camera.as_ref().is_some_and(|c| c.overlay.is_some()) {
                    tabs.push(OVERLAY_TAB);
                }
                tabs
            }
            Some(i) if i.texture_locator().is_some() => vec!["Texture Locator"],
            Some(i) if i.texture.is_some() => vec!["Group"],
            // GL-13: its own tab first, then the shared ones.
            Some(i) if i.is_group_locator() => vec![GROUP_LOCATOR_TAB],
            Some(i) if i.replicator.is_some() => vec![crate::replicator_ui::REPLICATOR_TAB],
            // ENVIRONMENT-PLACEMENT §2: only its own tab (Intensity, the
            // Visible to … switches and the LPE label); no Transform and
            // none of the shared item tabs.
            Some(i) if i.environment.is_some() => {
                return vec![crate::environment_ui::ENVIRONMENT_TAB];
            }
            Some(i) => match &i.light {
                Some(l) => vec![l.kind.tab_name()],
                None => vec!["Mesh", "Surface"],
            },
        };
        tabs.extend(shared);
        tabs
    }
    /// The active item's properties for one category tab: a mesh's Surface
    /// tab holds smoothing, its Mesh tab everything else.
    /// UIC-104: item properties start with a Name row, the same on every
    /// tab. Enter renames (one undo step); Escape puts the name back.
    fn item_name_row(&mut self, ui: &mut Ui) {
        let Some(name) = self.editor.scene.active().map(|i| i.name.clone()) else {
            return;
        };
        let edit = crate::form::rows::row(ui, "Name", |ui| {
            ui.add_sized(
                [ui.available_width(), crate::option_row::ROW_HEIGHT],
                TextEdit::singleline(&mut self.name)
                    .horizontal_align(Align::LEFT)
                    .background_color(palette::list())
                    .text_color(palette::field_text()),
            )
        })
        .inner
        .on_hover_text("The item's name; Enter renames it");
        self.text_editors.insert(edit.id);
        crate::numeric_tab::register(ui, &edit, Some(&self.name));
        if self.editor_input_allowed(ui.ctx())
            && (edit.has_focus() || edit.lost_focus())
            && ui.input(|i| i.key_pressed(Key::Enter))
        {
            let name = self.name.clone();
            self.rename_active(name);
            edit.surrender_focus();
        }
        if edit.has_focus() && ui.input(|i| i.key_pressed(Key::Escape)) {
            self.name = name;
            edit.surrender_focus();
        }
    }
    /// The selected items Item Transform edits: the primary first, then the
    /// others (UIC-56/57).
    pub(super) fn transform_targets(&self) -> Vec<operandi_core::Item> {
        self.common_targets()
            .iter()
            .filter_map(|id| self.editor.scene.item(*id))
            .cloned()
            .collect()
    }

    /// Writes `value` (shown units) to one transform channel of every
    /// selected, unlocked item as one undo step under `gesture`. `starts`
    /// adds it to each item's own start values instead (an arrow drag).
    /// `mode` spreads the edit over the group's other axes (UIC-56/57).
    pub(super) fn set_transform_channel(
        &mut self,
        prefix: &str,
        axis: usize,
        value: f64,
        starts: Option<&[(u64, [f64; 3])]>,
        mode: crate::link_mode::LinkMode,
        gesture: u64,
    ) -> std::result::Result<(), String> {
        let targets: Vec<operandi_core::Item> = self
            .transform_targets()
            .into_iter()
            .filter(|i| !i.is_locked())
            .collect();
        let previous = self.editor.history.undo_fold.replace(gesture);
        let mut result = Ok(());
        for target in targets {
            let now = [0, 1, 2].map(|a| channel(&target.transform, prefix, a));
            let (old, shown) = match starts {
                Some(starts) => match starts.iter().find(|(id, _)| *id == target.id) {
                    Some((_, start)) => (*start, start[axis] + value),
                    None => continue,
                },
                None => (now, value),
            };
            let new = mode.apply(old, axis, shown);
            if (0..3).all(|a| (now[a] - new[a]).abs() <= 1e-12) {
                continue;
            }
            let mut transform = target.transform.clone();
            for (a, v) in new.into_iter().enumerate() {
                match prefix {
                    "translation" => transform.translation[a] = v,
                    "rotation" => transform.rotation[a] = v.to_radians(),
                    _ => transform.scale[a] = v,
                }
            }
            if let Err(error) = self.editor.dispatch(Command::ItemTransform {
                item: target.id,
                transform,
            }) {
                result = Err(error);
                break;
            }
        }
        self.editor.history.undo_fold = previous;
        self.item_field_owner = None;
        result
    }

    /// Position, Rotation and Scale Factor rows over every selected item.
    fn item_transform_rows(
        &mut self,
        ui: &mut Ui,
        item: &operandi_core::Item,
        driven_rotation: bool,
    ) {
        let selected = self.transform_targets();
        for (prefix, label, quantity) in [
            ("translation", "Position", numeric::Quantity::Distance),
            ("rotation", "Rotation", numeric::Quantity::Angle),
            ("scale", "Scale Factor", numeric::Quantity::Scalar),
        ] {
            // A target or Physical Sun drives the rotation (LP-04, LP-SUN):
            // its fields grey.
            let driven = prefix == "rotation" && driven_rotation;
            for axis in 0..3 {
                let key = format!("{prefix}{axis}");
                // RG-04: a channel a rig piece drives greys, naming the piece.
                let group = match prefix {
                    "translation" => "pos",
                    "rotation" => "rot",
                    _ => "scl",
                };
                let rig_driver = operandi_core::rig::driven_by(
                    &self.editor.scene,
                    item.id,
                    &format!("{group}.{}", ["X", "Y", "Z"][axis]),
                )
                .map(|p| format!("Driven by {}", operandi_core::rig::label(&p.kind)));
                let current = channel(&item.transform, prefix, axis);
                ui.gated_ui(
                    crate::unavailable::unless(
                        !driven,
                        "Rotation follows the target or Physical Sun; turn that off to type it",
                    )
                    .or(rig_driver.as_deref()),
                    |ui| {
                        ui.horizontal(|ui| {
                            // UIC-112: keys show red, and the light keys
                            // (TK-20). Jason 2026-11-23: no
                            // changed-from-default fill on transform rows;
                            // animation lights cover it.
                            self.anim_row(
                                &format!(
                                    "{}.{}",
                                    match prefix {
                                        "translation" => "pos",
                                        "rotation" => "rot",
                                        _ => "scl",
                                    },
                                    ["X", "Y", "Z"][axis]
                                ),
                                false,
                            );
                            // UIC-114: "Position X", then "Y", "Z".
                            let shown = if axis == 0 {
                                format!("{label} X")
                            } else {
                                ["X", "Y", "Z"][axis].to_owned()
                            };
                            crate::option_row::label(ui, &shown);
                            let width = crate::form::rows::rest_width(ui).max(50.);
                            let (cell, _) = ui.allocate_exact_size(
                                vec2(width, crate::option_row::ROW_HEIGHT),
                                Sense::hover(),
                            );
                            let (arrows, text_rect) = crate::scrub::split(cell);
                            // UIC-56/114: the group's link button sits before
                            // the first row's mini-slider; the other rows keep
                            // the same field width.
                            let (link_rect, text_rect) =
                                crate::link_mode::LinkMode::split(text_rect);
                            let mode = self.link_modes.get(prefix).copied().unwrap_or_default();
                            if axis == 0 {
                                let link = crate::link_mode::button(ui, link_rect, mode);
                                self.field_rects
                                    .insert(format!("item-{prefix}-link"), link.rect);
                                if link.clicked() {
                                    self.link_modes.insert(prefix.into(), mode.next());
                                }
                            }
                            // UIC-53: the mini-slider arrows. Each selected item
                            // moves from its own value; one drag is one undo step.
                            let drag =
                                crate::scrub::handle(ui, arrows, &format!("item-{key}"), false);
                            self.field_rects
                                .insert(format!("item-{key}-arrows"), arrows);
                            let start_id = Id::new(("item-scrub-start", &key));
                            type Starts = (Vec<(u64, [f64; 3])>, crate::scrub::Scrub, u64);
                            if drag.drag_started() {
                                self.fold_serial = self.fold_serial.wrapping_add(1);
                                let gesture = 0x17E4_0000_0000_0000 ^ self.fold_serial;
                                let starts: Vec<(u64, [f64; 3])> = selected
                                    .iter()
                                    .map(|i| {
                                        (i.id, [0, 1, 2].map(|a| channel(&i.transform, prefix, a)))
                                    })
                                    .collect();
                                // UIC-54: detents follow the primary item's value.
                                let kind = match prefix {
                                    "translation" => crate::scrub::Kind::Distance,
                                    "rotation" => crate::scrub::Kind::Angle,
                                    _ => crate::scrub::Kind::Scalar,
                                };
                                let scrub =
                                    crate::scrub::Scrub::new(&key, kind, current, String::new());
                                ui.data_mut(|d| {
                                    d.insert_temp::<Starts>(start_id, (starts, scrub, gesture))
                                });
                            }
                            if drag.dragged()
                                && let Some((starts, mut scrub, gesture)) =
                                    ui.data(|d| d.get_temp::<Starts>(start_id))
                            {
                                // Shift is coarser; Option or Ctrl finer, as before.
                                let mut m = ui.input(|i| i.modifiers);
                                m.alt |= m.ctrl;
                                let delta = scrub.advance(drag.drag_delta().x, m);
                                ui.data_mut(|d| {
                                    d.insert_temp::<Starts>(
                                        start_id,
                                        (starts.clone(), scrub, gesture),
                                    )
                                });
                                if let Err(error) = self.set_transform_channel(
                                    prefix,
                                    axis,
                                    delta,
                                    Some(&starts),
                                    mode,
                                    gesture,
                                ) {
                                    self.status = error;
                                }
                            }
                            if drag.drag_stopped() {
                                ui.data_mut(|d| d.remove::<Starts>(start_id));
                            }
                            let response = ui.put(
                                text_rect,
                                TextEdit::singleline(
                                    self.item_fields.entry(key.clone()).or_default(),
                                )
                                .id_salt(("item", &key))
                                .horizontal_align(Align::LEFT)
                                .background_color(palette::list())
                                .text_color(palette::field_text()),
                            );
                            self.field_rects
                                .insert(format!("item-{key}"), response.rect);
                            self.text_editors.insert(response.id);
                            crate::numeric_tab::register(
                                ui,
                                &response,
                                self.item_fields.get(&key).map(String::as_str),
                            );
                            // Typing into "(mixed)" starts from empty.
                            if response.gained_focus()
                                && self.item_fields.get(&key).map(String::as_str)
                                    == Some(crate::form::rows::MIXED)
                            {
                                self.item_fields.insert(key.clone(), String::new());
                            }
                            if self.field_errors.contains_key(&format!("item-{key}")) {
                                ui.painter().rect_stroke(
                                    response.rect,
                                    1.,
                                    Stroke::new(1.0f32, theme::error()),
                                    StrokeKind::Inside,
                                );
                            }
                            if !self.editor_input_allowed(ui.ctx()) {
                                return;
                            }
                            if response.has_focus() && ui.input(|i| i.key_pressed(Key::Escape)) {
                                self.item_field_owner = None;
                                self.field_errors.remove(&format!("item-{key}"));
                                response.surrender_focus();
                                return;
                            }
                            if (response.has_focus() || response.lost_focus())
                                && ui.input(|i| i.key_pressed(Key::Enter))
                            {
                                let text = self.item_fields[&key].clone();
                                if text == crate::form::rows::MIXED {
                                    // Untouched "(mixed)": nothing to write.
                                    response.surrender_focus();
                                    return;
                                }
                                self.fold_serial = self.fold_serial.wrapping_add(1);
                                let gesture = 0x17E5_0000_0000_0000 ^ self.fold_serial;
                                // UIC-57: how Enter is pressed picks the link
                                // mode for this one entry.
                                let entry_mode = crate::link_mode::LinkMode::for_enter(
                                    ui.input(|i| i.modifiers),
                                    mode,
                                );
                                let result =
                                    numeric::parse_quantity(&text, quantity).and_then(|value| {
                                        self.set_transform_channel(
                                            prefix, axis, value, None, entry_mode, gesture,
                                        )
                                    });
                                match result {
                                    Ok(()) => {
                                        self.field_errors.remove(&format!("item-{key}"));
                                        self.status = "Item transform updated".into();
                                        response.surrender_focus();
                                    }
                                    Err(error) => {
                                        self.field_errors
                                            .insert(format!("item-{key}"), error.clone());
                                        self.status = error;
                                        response.request_focus();
                                    }
                                }
                            }
                        })
                    },
                );
            }
        }
    }

    fn item_properties(&mut self, ui: &mut Ui, tab: &str) {
        self.item_properties_view(ui, tab, false);
    }

    /// MP-03: the selected item's short view for Mini Properties: Name and
    /// the Item Transform (Position, Rotation, Scale).
    pub(super) fn item_mini_properties(&mut self, ui: &mut Ui) {
        self.item_properties_view(ui, "", true);
    }

    fn item_properties_view(&mut self, ui: &mut Ui, tab: &str, short: bool) {
        self.item_name_row(ui);
        if tab == "Surface" {
            self.smoothing_properties(ui);
            return;
        }
        if tab == crate::item_common_ui::DISPLAY_TAB {
            self.display_properties(ui);
            self.display_extra_rows(ui);
            self.camera_display_rows(ui);
            return;
        }
        if tab == crate::camera_props_ui::EFFECTS_TAB {
            self.camera_effects_tab(ui);
            return;
        }
        if tab == crate::camera_props_ui::OVERLAY_TAB {
            self.camera_overlay_tab(ui);
            return;
        }
        if tab == crate::item_common_ui::ASSEMBLY_TAB {
            self.assembly_properties(ui);
            self.assembly_extra_rows(ui);
            return;
        }
        if tab == crate::item_extras_ui::CHANNELS_TAB {
            self.user_channels_tab(ui);
            return;
        }
        if tab == crate::item_extras_ui::TAGS_TAB {
            self.tags_tab(ui);
            return;
        }
        let Some(item) = self.editor.scene.active().cloned() else {
            ui.weak("Select a mesh");
            self.item_field_owner = None;
            return;
        };
        // ENVIRONMENT-PLACEMENT §2: an environment has no Item Transform.
        if item.environment.is_some() {
            self.item_field_owner = None;
            self.environment_item_rows(ui, item.id);
            return;
        }
        // UIC-56/57/118: the fields show every selected item; "(mixed)"
        // where they differ. Refreshed when any of their transforms change.
        let selected = self.transform_targets();
        let signature: Vec<(u64, operandi_core::scene::Transform)> = selected
            .iter()
            .map(|i| (i.id, i.transform.clone()))
            .collect();
        let signature_id = Id::new("item-fields-selection");
        let same_selection = ui.data(|d| {
            d.get_temp::<Vec<(u64, operandi_core::scene::Transform)>>(signature_id)
                .is_some_and(|s| s == signature)
        });
        if self.item_field_owner.as_ref() != Some(&(item.id, item.transform.clone()))
            || !same_selection
        {
            self.item_fields.clear();
            for prefix in ["translation", "rotation", "scale"] {
                for axis in 0..3 {
                    let values: Vec<f64> = selected
                        .iter()
                        .map(|i| channel(&i.transform, prefix, axis))
                        .collect();
                    let first = channel(&item.transform, prefix, axis);
                    let text = if values.iter().any(|v| (v - first).abs() > 1e-12) {
                        crate::form::rows::MIXED.to_owned()
                    } else {
                        number_text(first, channel_unit(prefix))
                    };
                    self.item_fields.insert(format!("{prefix}{axis}"), text);
                }
            }
            self.item_field_owner = Some((item.id, item.transform.clone()));
            ui.data_mut(|d| d.insert_temp(signature_id, signature));
        }
        // RG-04: a field a rig piece drives shows the evaluated value
        // (greyed below), not the hidden own value.
        let driven: Vec<(&str, usize)> = ["translation", "rotation", "scale"]
            .into_iter()
            .flat_map(|prefix| (0..3).map(move |axis| (prefix, axis)))
            .filter(|(prefix, axis)| {
                let group = match *prefix {
                    "translation" => "pos",
                    "rotation" => "rot",
                    _ => "scl",
                };
                operandi_core::rig::driven_by(
                    &self.editor.scene,
                    item.id,
                    &format!("{group}.{}", ["X", "Y", "Z"][*axis]),
                )
                .is_some()
            })
            .collect();
        if !driven.is_empty() {
            let baked = operandi_core::rig_eval::baked_scene(&self.editor.scene);
            if let Some(t) = baked.item(item.id).map(|i| i.transform.clone()) {
                for (prefix, axis) in driven {
                    self.item_fields.insert(
                        format!("{prefix}{axis}"),
                        number_text(channel(&t, prefix, axis), channel_unit(prefix)),
                    );
                }
            }
        }
        crate::form::rows::section(ui, "Item Transform", false, |ui| {
            // IT-PHY-01: changing the order keeps the angles as typed.
            let mut order = item.transform.rotation_order();
            // CP-A02: a locked item's transform fields are greyed.
            let not_editable = crate::unavailable::first([
                crate::unavailable::unless(
                    self.editor.live.is_none(),
                    crate::unavailable::TOOL_ACTIVE,
                ),
                crate::unavailable::unless(!item.is_locked(), crate::unavailable::LOCKED),
            ]);
            let driven_rotation = self.editor.scene.rotation_driven(item.id);
            ui.gated_ui(not_editable, |ui| {
                crate::option_row::choice(
                    ui,
                    "item-rotation-order",
                    "Rotation Order",
                    &mut order,
                    &operandi_core::scene::RotationOrder::ALL.map(|o| (o, o.label())),
                );
            });
            // The row runs item.channel rotOrder (UIC-164/165).
            if order != item.transform.rotation_order() {
                self.run_command_line(&format!(
                    "item.channel rotOrder {} item:{}",
                    order.label(),
                    item.id
                ));
            }
            ui.weak("Scale: Factor");
            ui.gated_ui(not_editable, |ui| {
                ui.spacing_mut().item_spacing.y = crate::form::layout::panel::ROW_GAP;
                self.item_transform_rows(ui, &item, driven_rotation);
            });
            // Spec freeze-transforms section 1: Reset and Freeze directly
            // under the transform rows (gated on their own).
            self.transform_action_rows(ui, &item);
        });
        if short {
            return;
        }
        if item.backdrop.is_some() {
            self.backdrop_properties(ui, &item);
        } else if item.camera.is_some() {
            self.camera_view_tab(ui, &item);
        } else if item.light.is_some() {
            self.light_properties(ui, &item);
        } else if item.texture_locator().is_some() {
            // Its projection settings sit with the layers that use it.
            ui.weak(
                "Texture Locator: its projection settings are on the Texture Locator \
                 tab of a texture layer that uses it (Shading).",
            );
        } else if item.texture.is_some() {
            ui.weak("Holds the scene's Texture Locators.");
        } else if item.is_group_locator() {
            ui.weak("Moves, rotates and scales the items parented under it.");
        } else if item.replicator.is_some() {
            self.replicator_properties(ui, &item);
        } else {
            self.subdivision_properties(ui, &item);
        }
    }
    /// SS-01 smoothing of the scene's default material, which shades every
    /// mesh. A field drag is one undo step.
    fn smoothing_properties(&mut self, ui: &mut Ui) {
        use operandi_core::shading::{
            SMOOTHING_AMOUNT_RANGE, SMOOTHING_ANGLE_RANGE, SmoothingGroups,
        };
        crate::form::rows::section(ui, "Smoothing", true, |ui| {
            let current = self.editor.scene.smoothing.clone();
            let mut next = current.clone();
            let mut continuing = false;
            ui.gated_ui(crate::unavailable::unless(self.editor.live.is_none(), crate::unavailable::TOOL_ACTIVE), |ui| {
                    ui.weak("Default material: applies to every mesh");
                    ui.horizontal(|ui| {
                        crate::option_row::label(ui, "Smoothing");
                        crate::option_row::fill(ui, |ui| {
                            let r = ui
                                .add(
                                    crate::numeric_tab::DragValue::new(&mut next.amount)
                                        .range(SMOOTHING_AMOUNT_RANGE)
                                        .speed(0.5)
                                        .suffix(" %"),
                                )
                                .on_hover_text(
                                    "0 % flat, 100 % fully smooth; above overshoots (provisional)",
                                );
                            continuing |= r.dragged() && !r.drag_started();
                        })
                    });
                    ui.horizontal(|ui| {
                        crate::option_row::label(ui, "Smoothing Angle");
                        crate::option_row::fill(ui, |ui| {
                            let r = ui
                                .add(
                                    crate::numeric_tab::DragValue::new(&mut next.angle)
                                        .range(SMOOTHING_ANGLE_RANGE)
                                        .speed(0.25)
                                        .suffix("°"),
                                )
                                .on_hover_text(
                                    "Neighbouring polygons within this angle blend; sharper edges stay hard",
                                );
                            continuing |= r.dragged() && !r.drag_started();
                        })
                    });
                    crate::option_row::check(ui, "Crease by Smoothing Angle", &mut next.crease_by_angle)
                        .on_hover_text("Off: every connected polygon blends, except across smoothing groups");
                    crate::option_row::choice(
                        ui,
                        "smoothing-groups",
                        "Smoothing Groups",
                        &mut next.groups,
                        &[
                            (SmoothingGroups::Off, "Off"),
                            (SmoothingGroups::Crease, "Crease"),
                            (SmoothingGroups::Force, "Force"),
                        ],
                    );
                });
            if next != current {
                self.run(Command::SetSmoothing {
                    settings: next,
                    continuing,
                });
            }
        });
    }
    /// SDV-03 per-mesh subdivision settings. Each change is one undo step.
    fn subdivision_properties(&mut self, ui: &mut Ui, item: &operandi_core::Item) {
        use operandi_core::subdivision::{BoundaryRule, MAX_LEVEL};
        crate::form::rows::section(ui, "Subdivision", true, |ui| {
            let current = item.subdivision.clone().unwrap_or_default();
            let mut next = current.clone();
            ui.gated_ui(
                crate::unavailable::unless(
                    self.editor.live.is_none(),
                    crate::unavailable::TOOL_ACTIVE,
                ),
                |ui| {
                    for (label, level, hint) in [
                        (
                            "Subdivision Level",
                            &mut next.subdivision_level,
                            "Tab polygons. Above level 1 the surface is provisional (SDV-Q01).",
                        ),
                        (
                            "Catmull-Clark Level",
                            &mut next.catmull_clark_level,
                            "Shift+Tab polygons",
                        ),
                        (
                            "Subdivision Render Level",
                            &mut next.subdivision_render_level,
                            "Tab polygons in Cycles renders",
                        ),
                        (
                            "Catmull-Clark Render Level",
                            &mut next.catmull_clark_render_level,
                            "Shift+Tab polygons in Cycles renders",
                        ),
                    ] {
                        ui.horizontal(|ui| {
                            crate::option_row::label(ui, label);
                            crate::option_row::fill(ui, |ui| {
                                ui.add(
                                    crate::numeric_tab::DragValue::new(level)
                                        .range(0..=MAX_LEVEL)
                                        .speed(0.05),
                                )
                                .on_hover_text(hint);
                            })
                        });
                    }
                    // G7 PC-7: cells along each side of a live patch.
                    ui.horizontal(|ui| {
                        crate::option_row::label(ui, "Patch Subdivisions");
                        crate::option_row::fill(ui, |ui| {
                            ui.add(
                                crate::numeric_tab::DragValue::new(&mut next.patch_subdivisions)
                                    .range(1..=operandi_core::patch_live::MAX_CELLS)
                                    .speed(0.1),
                            )
                            .on_hover_text("Cells along each side of a live patch, drawn, rendered and frozen");
                        })
                    });
                    crate::option_row::choice(
                        ui,
                        "subdivision-boundary-rule",
                        "Boundary Rule",
                        &mut next.boundary_rule,
                        &[
                            (BoundaryRule::Crease, "Crease"),
                            (BoundaryRule::Smooth, "Smooth"),
                            (BoundaryRule::CreaseEdges, "Crease Edges"),
                        ],
                    );
                    let can_freeze = operandi_core::subdivision::has_smooth(&item.mesh)
                        || operandi_core::patch_live::has_patches(&item.mesh);
                    if ui
                        .gated(
                            crate::unavailable::unless(
                                can_freeze,
                                "The mesh has no subdivision surface polygons or live patches to freeze",
                            ),
                            Button::new("Freeze"),
                        )
                        .on_hover_text(
                            "Freeze curves and smooth polygons into plain geometry, with the last Freeze settings",
                        )
                        .clicked()
                    {
                        self.run(Command::Freeze(self.subdivide_ui.freeze_options.core()));
                    }
                },
            );
            if next != current {
                self.run(Command::SetSubdivision(next));
            }
        });
    }
    pub(super) fn command_entry(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Command");
            let response = ui.add(
                TextEdit::singleline(&mut self.command)
                    .id(Id::new("command-entry"))
                    .font(FontId::proportional(11.))
                    .desired_width(ui.available_width())
                    .hint_text("Tool, Undo, Redo, pen {…}…"),
            );
            self.text_editors.insert(response.id);
            crate::numeric_tab::register(ui, &response, Some(&self.command));
            if response.has_focus() && ui.input(|i| i.key_pressed(Key::Escape)) {
                response.surrender_focus();
            }
            // HIS-61: Up/Down step through earlier typed lines.
            if response.has_focus() {
                if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowUp)) {
                    self.recall(true);
                } else if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown)) {
                    self.recall(false);
                }
            }
            if self.editor_input_allowed(ui.ctx())
                && (response.has_focus() || response.lost_focus())
                && ui.input(|i| i.key_pressed(Key::Enter))
            {
                let command = std::mem::take(&mut self.command);
                if command.trim().starts_with("pen ")
                    || command::canonical_pen_payload(&command).is_some()
                {
                    self.command = command.clone();
                }
                // HIS-60: the line joins the history (and the undo list
                // when undoable) as typed.
                self.remember_typed(&command);
                self.execute_from(operandi_core::Source::Typed, &command);
                response.surrender_focus();
            }
        });
    }
    pub(super) fn action_context_choice(
        &mut self,
        ui: &mut Ui,
        button: Button<'_>,
        context: ActionContext,
    ) -> Response {
        let error = if self.field_errors.is_empty() {
            self.editor.validate_action_context(context).err()
        } else {
            Some("Correct or revert invalid numeric fields before changing Action Center".into())
        };
        let response = ui.gated(error.as_deref(), button);
        if response.clicked() {
            self.choose_action_context(context);
        }
        response
    }
    /// An Action Center chosen by name (the menu, a pie). A Snaps and
    /// Precision preset still waiting for its centre click stops waiting:
    /// the choice is the centre now, and a later press mustn't replace it.
    pub(super) fn choose_action_context(&mut self, context: ActionContext) {
        self.preset_center_click = false;
        self.run(Command::ActionContext(context));
    }
    pub(super) fn context_bar(&mut self, ui: &mut Ui) {
        // N31 (Jason, 2026-11-17): the button names the action centre in use
        // and is lit (UIC-01) whenever it isn't None.
        let current = self.editor.action_context;
        let set = current != ActionContext::default();
        let label = if set {
            format!("Action Center: {}", current.center.label())
        } else {
            "Action Center".to_owned()
        };
        egui::containers::menu::MenuButton::from_button(
            icons::button(ui, icons::Icon::ActionCenter, &label).selected(set),
        )
        .ui(ui, |ui| {
            self.menu_owned_input = true;
            let current = self.editor.action_context;
            ui.label(format!(
                "Center: {} · Axis: {}",
                current.center.label(),
                current.axis.label()
            ));
            if self.editor.live.is_some() && self.editor.validate_action_context(current).is_err() {
                ui.weak("Drop or Cancel the current tool to change its center or axes.");
            }
            self.action_context_choice(
                ui,
                Button::selectable(current == ActionContext::default(), "None"),
                ActionContext::default(),
            );
            let automatic = ActionContext {
                center: CenterMode::Automatic,
                axis: AxisOrientation::World,
            };
            self.action_context_choice(
                ui,
                Button::selectable(current == automatic, "Automatic"),
                automatic,
            )
            .on_hover_text("Centre of the bounding box of what the tool moves (the whole mesh with nothing selected)");
            // action-center-scale-2026-11-06 F2/F3.
            for center in [CenterMode::SelectionBorder, CenterMode::SelectionAutoAxis] {
                // AC-21: Selection Border takes the selection's axes;
                // AC-22: Auto Axis keeps World's.
                let axis = if center == CenterMode::SelectionBorder {
                    AxisOrientation::Selection
                } else {
                    AxisOrientation::World
                };
                let context = ActionContext { center, axis };
                // Tooltips per UIC-04.
                let tip = if center == CenterMode::SelectionBorder {
                    "Middle of the edges where the selected polygons meet the rest of the mesh, with axes that follow the selection's shape (X along its length)"
                } else {
                    "Center of the selection, with the World axes"
                };
                self.action_context_choice(
                    ui,
                    Button::selectable(current.center == center, center.label()),
                    context,
                )
                .on_hover_text(tip);
            }
            // C-ELEM, C-SCREEN: in force when chosen; every press off the
            // handle then picks or places. Tooltips per UIC-04.
            for (label, tip) in [
                (
                    "Element",
                    "The polygon, edge or point you press with a transform tool on, with axes from its normal. Press another to move there",
                ),
                (
                    "Screen",
                    "Where you press in the view with a transform tool on, with axes that face the view",
                ),
            ] {
                let on = current.center.label() == label;
                let error = (!self.field_errors.is_empty()).then_some(
                    "Correct or revert invalid numeric fields before changing Action Center",
                );
                if ui
                    .gated(error, Button::selectable(on, label))
                    .on_hover_text(tip)
                    .clicked()
                {
                    if label == "Element" {
                        self.choose_element_center();
                    } else {
                        self.choose_screen_center();
                    }
                    ui.close();
                }
            }
            let selection = ActionContext {
                center: CenterMode::Selection,
                axis: AxisOrientation::Selection,
            };
            self.action_context_choice(
                ui,
                Button::new("Selection").selected(current == selection),
                selection,
            )
            .on_hover_text(
                // A-SEL (AC-RULES v6); tooltip per UIC-04.
                "Center of the selection, with axes that follow its shape: X along its longest direction, Z through its thinnest (a flat patch's normal)",
            );
            let origin = ActionContext {
                center: CenterMode::Origin,
                axis: AxisOrientation::World,
            };
            self.action_context_choice(ui, Button::selectable(current == origin, "Origin"), origin);
            let local = ActionContext {
                center: CenterMode::Local,
                axis: AxisOrientation::World,
            };
            self.action_context_choice(
                ui,
                Button::selectable(current.center == CenterMode::Local, "Local"),
                local,
            )
            .on_hover_text("Each connected piece about its own centre, with axes from its own polygons");
            // The rulebook's item rows (ACS table, pivot_service.rs): one
            // centre rule plus one axis rule each. Tooltips per UIC-04.
            for (label, center, axis, tip) in [
                (
                    "Parent",
                    CenterMode::Parent,
                    AxisOrientation::Parent,
                    "The parent item's pivot, with the parent's axes",
                ),
                (
                    "Pivot",
                    CenterMode::Pivot,
                    AxisOrientation::Item,
                    "The mesh item's own pivot, with the item's axes",
                ),
                (
                    "Pivot Center Parent Axis",
                    CenterMode::Pivot,
                    AxisOrientation::Parent,
                    "The mesh item's own pivot, with the parent's axes",
                ),
            ] {
                let context = ActionContext { center, axis };
                self.action_context_choice(
                    ui,
                    Button::selectable(current == context, label),
                    context,
                )
                .on_hover_text(tip);
            }
            ui.separator();
            ui.menu_button("Center", |ui| {
                for center in [
                    CenterMode::None,
                    CenterMode::Selection,
                    CenterMode::Origin,
                    CenterMode::Automatic,
                ] {
                    let context = ActionContext {
                        center,
                        ..self.editor.action_context
                    };
                    self.action_context_choice(
                        ui,
                        Button::selectable(
                            self.editor.action_context.center == center,
                            center.label(),
                        ),
                        context,
                    );
                }
            });
            ui.menu_button("Axis", |ui| {
                for axis in [
                    AxisOrientation::World,
                    AxisOrientation::Workplane(self.actual_plane()),
                    AxisOrientation::Selection,
                ] {
                    let context = ActionContext {
                        axis,
                        ..self.editor.action_context
                    };
                    self.action_context_choice(
                        ui,
                        Button::new(axis.label()).selected(self.editor.action_context.axis == axis),
                        context,
                    );
                }
                ui.weak("Explicit major-plane frame; held for this tool task");
            });
            ui.weak("Center and axis changes update the active tool.");
        });
        let label = format!(
            "Falloff: {}",
            self.editor.spatial_falloff.map_or("None", |f| f.label())
        );
        let response = ui.add(icons::button(ui, icons::Icon::Falloff, &label));
        Popup::menu(&response).show(|ui| self.falloff_menu(ui));
        self.symmetry_menu(ui);
        // MC-03: the Mesh Constraint button, its own switch beside
        // Snapping's (MC-02).
        self.mesh_constraint_button(ui);
        // SN-01: the snapping switch. Holding Option turns it into an
        // Options button for Snapping Options (Jason's QA); right-click
        // opens them too.
        let options = ui.input(|i| i.modifiers.alt);
        // Jason (2026-11-14): the label stays "Snapping" so the toolbar
        // doesn't grow; on/off is the button's accent colour (UIC-23), and
        // Option swaps it to Options (UIC-27).
        let label = if options { "Options" } else { "Snapping" };
        // UIC-04: name and shortcut, the usage hint and the Option
        // alternate, then the active targets one per line.
        let active = self.snapping.active(self.editor.scene.selection.mode);
        let targets: Vec<&str> = crate::snap_kinds::Kind::ALL
            .iter()
            .filter(|&&k| active.get(k))
            .map(|k| k.label())
            .collect();
        let state = if self.snapping.enabled { "on" } else { "off" };
        let list = if targets.is_empty() {
            "  (none ticked)".to_owned()
        } else {
            targets
                .iter()
                .map(|t| format!("  {t}"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let tip = format!(
            "Snapping{} · {state}\nTurn snapping on or off · hold Option and click for Snapping \
             Options{}\n\nSnap targets:\n{list}",
            self.shortcut("Snapping Toggle"),
            self.shortcut("Snapping Options")
        );
        let snapping = ui
            .add(
                icons::button(ui, icons::Icon::Snapping, label)
                    .selected(self.snapping.enabled && !options),
            )
            .on_hover_text(tip);
        if snapping.clicked() {
            if options {
                // Below the button, like a popover.
                self.snapping_options = Some(snapping.rect.left_bottom() + vec2(0., 4.));
                self.snapping_entered = false;
                self.snapping_opening = true;
            } else {
                self.toggle_snapping();
            }
        }
        if snapping.secondary_clicked() {
            self.open_snapping_options();
        }
        let workplane = icons::button(ui, icons::Icon::Workplane, "Work Plane…");
        if ui
            .add(workplane)
            .on_hover_text("Work plane construction and display settings")
            .clicked()
        {
            self.show_plane = !self.show_plane;
        }
        // DA-1: Drop Action, after Work Plane and before Render.
        self.drop_action_menu(ui);
        // Render (F9) and Preview (F8), right of Work Plane (Jason's QA).
        self.render_buttons(ui);
    }
    /// SEL-IT-01..03: Parent To and Unparent, each with or without keeping
    /// the item's world position.
    pub(super) fn parent_menu(&mut self, ui: &mut Ui, item: u64, parent: Option<u64>) {
        let idle = self.editor.live.is_none();
        let others: Vec<(u64, String)> = self
            .editor
            .scene
            .items
            .iter()
            // ENVIRONMENT-PLACEMENT §1: environments aren't parents.
            .filter(|i| i.id != item && i.in_item_list())
            .map(|i| (i.id, i.name.clone()))
            .collect();
        for (title, keep_position) in [("Parent To", false), ("Parent To, Keep Position", true)] {
            ui.gated_ui(
                crate::unavailable::first([
                    crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE),
                    crate::unavailable::unless(
                        !others.is_empty(),
                        "The scene has no other item to parent to",
                    ),
                ]),
                |ui| {
                    ui.menu_button(title, |ui| {
                        for (id, name) in &others {
                            if ui.button(name).clicked() {
                                self.run(Command::SetParent {
                                    item,
                                    parent: Some(*id),
                                    keep_position,
                                });
                                ui.close();
                            }
                        }
                    });
                },
            );
        }
        for (title, keep_position) in [("Unparent", false), ("Unparent, Keep Position", true)] {
            if ui
                .gated(
                    crate::unavailable::first([
                        crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE),
                        crate::unavailable::unless(parent.is_some(), "The item has no parent"),
                    ]),
                    Button::new(title),
                )
                .clicked()
            {
                self.run(Command::SetParent {
                    item,
                    parent: None,
                    keep_position,
                });
                ui.close();
            }
        }
        // GL-04: the selection (this row joins it if it isn't in it).
        if ui
            .gated(
                crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE),
                Button::new(PARENT_TO_GROUP_LOCATOR),
            )
            .on_hover_text("Put the selected items under a new Group Locator")
            .clicked()
        {
            if !self.editor.scene.selected_items().contains(&item) {
                self.run(Command::SelectItem(Some(item)));
            }
            self.run(Command::ParentToGroupLocator);
            ui.close();
        }
    }
    /// Symmetry setting (SYM-ST-01) and its commands. Arbitrary axis,
    /// Work Plane, Topology and UV symmetry are not built (SYM-Q05/06/11,
    /// SYM-SEL-06).
    fn symmetry_menu(&mut self, ui: &mut Ui) {
        let current = self.editor.symmetry;
        let label = format!("Symmetry: {}", current.label());
        // UIC-01/UIC-23: lit in the accent colour while Symmetry is on, like
        // the Snapping button (Jason, 2026-11-14).
        egui::containers::menu::MenuButton::from_button(
            icons::button(ui, icons::Icon::Symmetry, &label).selected(current.axis.is_some()),
        )
        .ui(ui, |ui| {
            self.menu_owned_input = true;
            ui.horizontal(|ui| {
                for axis in [None, Some(0), Some(1), Some(2), Some(3)] {
                    let choice = operandi_core::Symmetry { axis, ..current };
                    if ui
                        .selectable_label(current.axis == axis, choice.label())
                        .clicked()
                    {
                        self.run(Command::Symmetry(choice));
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label("Offset");
                let mut offset = current.offset;
                let response = ui
                    .add(crate::numeric_tab::DragValue::new(&mut offset).speed(0.01).suffix(" m"))
                    .on_hover_text(
                        "Moves the mirror plane along its axis, in the mesh's own coordinates",
                    );
                if response.changed() && offset != current.offset {
                    self.run(Command::Symmetry(operandi_core::Symmetry {
                        offset,
                        ..current
                    }));
                }
            });
            if current.axis == Some(3) {
                ui.horizontal(|ui| {
                    ui.label("Axis vector");
                    let mut vector = current.vector;
                    let mut changed = false;
                    for k in 0..3 {
                        changed |= ui.add(crate::numeric_tab::DragValue::new(&mut vector[k]).speed(0.01)).changed();
                    }
                    if changed && vector.length() > 1e-9 {
                        self.run(Command::Symmetry(operandi_core::Symmetry { vector, ..current }));
                    }
                });
            }
            let mut work_plane = current.work_plane;
            if ui
                .checkbox(&mut work_plane, "Work Plane")
                .on_hover_text("The mirror plane passes through the work plane centre")
                .changed()
            {
                self.run(Command::Symmetry(operandi_core::Symmetry { work_plane, ..current }));
            }
            let mut topology = current.topology;
            if ui
                .checkbox(&mut topology, "Topology")
                .on_hover_text("Find mirror partners through the mesh's connectivity, so slightly moved points still match")
                .changed()
            {
                self.run(Command::Symmetry(operandi_core::Symmetry { topology, ..current }));
            }
            // N22: the unpaired-point markers on or off (a display
            // preference, also on Preferences > Display).
            let mut shown = self.viewport_preferences.show_asymmetries;
            if ui
                .checkbox(&mut shown, "Show Asymmetries")
                .on_hover_text("Mark points with no mirror partner with small yellow squares")
                .changed()
            {
                self.viewport_preferences.show_asymmetries = shown;
                if let Err(error) = self
                    .viewport_preferences
                    .save(&self.environment.paths().viewport_display())
                {
                    self.status = format!("Preference changed; could not save it: {error}");
                }
            }
            let toggle = format!("Toggle Symmetry{}", self.shortcut("Toggle Symmetry"));
            if ui
                .button(toggle)
                .on_hover_text("Turn Symmetry off, or back on with the last axis")
                .clicked()
            {
                self.run(Command::ToggleSymmetry);
            }
            ui.separator();
            let idle = !self.editor_tool_active();
            let on = current.axis.is_some();
            let selection = self.editor.scene.selection.mode != Mode::Item;
            for (name, command, enabled) in [
                (
                    "Select Symmetrical",
                    Command::SelectSymmetrical,
                    idle && on && selection,
                ),
                (
                    "Symmetrize Positive",
                    Command::Symmetrize(operandi_core::SymmetrizeSide::Positive),
                    idle && on,
                ),
                (
                    "Symmetrize Negative",
                    Command::Symmetrize(operandi_core::SymmetrizeSide::Negative),
                    idle && on,
                ),
                // Section 9: rebuild one half as the mirror of the other.
                (
                    "Symmetrize Polygons Positive",
                    Command::SymmetrizePolygons {
                        side: operandi_core::SymmetrizeSide::Positive,
                        merge: true,
                    },
                    idle && on,
                ),
                (
                    "Symmetrize Polygons Negative",
                    Command::SymmetrizePolygons {
                        side: operandi_core::SymmetrizeSide::Negative,
                        merge: true,
                    },
                    idle && on,
                ),
                // Section 9: drag a Threshold to snap near pairs together.
                (
                    "Symmetry Tool",
                    Command::Activate(Tool::SymmetryTool),
                    idle && on && selection,
                ),
            ] {
                let text = format!("{name}{}", self.shortcut(name));
                let why = crate::unavailable::first([
                    crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE),
                    crate::unavailable::unless(on, "Turn Symmetry on (X, Y or Z) first"),
                    crate::unavailable::unless(enabled, "Switch to a component mode first"),
                ]);
                let response = ui.gated(why, Button::new(text));
                if response.clicked() {
                    self.run(command);
                    ui.close();
                }
            }
        });
    }
    pub(super) fn workspace_dialogs(&mut self, ctx: &Context) {
        let mut open = self.show_plane;
        if open {
            let (still_open, _) = self.palette_window(ctx, "work-plane", "Work Plane Settings", [330., 420.], None, |app, ui| {
                    use crate::form::{Unit, rows};
                    ui.set_width(300.);
                    rows::scope(ui, "work-plane-settings", |ui| {
                        let mut plane = app.editor.scene.workplane.clone();
                        rows::status(plane.fixed);
                        rows::check(ui, "Fixed", &mut plane.fixed)
                            .on_hover_text("Keep the work plane where it is instead of following the view");
                        // UIC-113: a choice is a dropdown here; picking a
                        // plane fixes it.
                        let mut axis = if plane.fixed {
                            plane.plane
                        } else {
                            app.actual_plane()
                        };
                        let shown = axis;
                        rows::choice_d(
                            ui,
                            "Plane",
                            &mut axis,
                            &shown,
                            &[(0, "XZ"), (1, "YZ"), (2, "XY")],
                            "The major plane the work plane lies in",
                        );
                        if axis != shown {
                            plane.plane = axis;
                            plane.fixed = true;
                        }
                        if rows::button(ui, "Reset / Automatic")
                            .on_hover_text("Back to the automatic plane through the origin")
                            .clicked()
                        {
                            plane.fixed = false;
                            plane.center = DVec3::ZERO;
                        }
                        if plane != app.editor.scene.workplane {
                            app.run(Command::Workplane(plane));
                        }
                        rows::section(ui, "Display", false, |ui| {
                            rows::check(ui, "Ground Grid", &mut app.display.settings.drawing.grid)
                                .on_hover_text("Draw the ground grid in this view");
                            rows::check(ui, "Ground X/Z Axes", &mut app.ground_axes)
                                .on_hover_text("World X and Z axes, fading toward the viewport edges. Independent of workplane opacity.");
                            if rows::check(
                                ui,
                                "Show Work Plane",
                                &mut app.viewport_preferences.workplane_display,
                            )
                            .on_hover_text("Draw the work plane's own grid, apart from the ground grid")
                            .changed()
                                && let Err(error) = app
                                    .viewport_preferences
                                    .save(&app.environment.paths().viewport_display())
                            {
                                app.status = format!(
                                    "Work plane display changed; could not save display preference: {error}"
                                );
                            }
                            let d = crate::viewport_preferences::ViewportPreferences::default();
                            if rows::number_d(
                                ui,
                                "Work Plane Opacity",
                                &mut app.viewport_preferences.workplane_opacity,
                                d.workplane_opacity,
                                Unit::Percent,
                                0..=100,
                                "Workplane grid opacity. Ground grid and tool handles are unaffected.",
                            )
                            .changed()
                                && let Err(error) = app
                                    .viewport_preferences
                                    .save(&app.environment.paths().viewport_display())
                            {
                                app.status = format!(
                                    "Opacity changed; could not save display preference: {error}"
                                );
                            }
                            rows::check(ui, "Work Plane Axes", &mut app.axes)
                                .on_hover_text("Draw the work plane's own axes");
                            rows::check(ui, "Actual Plane Badge", &mut app.plane_widget)
                                .on_hover_text("Show which plane the work plane is on");
                        });
                        rows::note(ui, "Origin-locked major planes. Display does not enable snapping.");
                    });
                });
            open = open && still_open;
        }
        self.show_plane = open;
        let mut open = self.show_diagnostics;
        if open {
            let (still_open, _) = self.palette_window(
                ctx,
                "diagnostics",
                "Diagnostics",
                [520., 360.],
                None,
                |app, ui| {
                    ui.label(&app.gpu);
                    let font = crate::typography::report(ctx);
                    ui.label(format!(
                        "UI font: {} {} · size {} · source {} · face {:?}",
                        font.family, font.face, font.standard_size, font.source, font.face_index
                    ));
                    ui.label(format!(
                        "Fallback: {}",
                        font.fallback
                            .as_deref()
                            .unwrap_or("none (missing glyphs use egui defaults)")
                    ));
                    ui.label(format!(
                        "Scale: {} pixels/point · native {:?}",
                        ctx.pixels_per_point(),
                        ctx.input(|i| i.viewport().native_pixels_per_point)
                    ));
                    ui.label(format!("Viewport drawable: {:?}", app.last_drawable));
                    ui.label("Modeling development build · feature acceptance remains bounded");
                    ui.label(format!(
                        "Executable: {}",
                        std::env::current_exe()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default()
                    ));
                },
            );
            open = open && still_open;
        }
        self.show_diagnostics = open;
    }
    pub(super) fn load_layout(&mut self, ctx: &Context) {
        let path = self.environment.paths().layout();
        self.load_layout_from(ctx, &path);
    }
    /// Apply panel widths and the Scenes/Properties split from a layout file.
    pub(super) fn load_layout_from(&mut self, ctx: &Context, path: &std::path::Path) {
        if let Ok(bytes) = std::fs::read(path)
            && let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes)
        {
            self.scene_fraction = value["split"].as_f64().unwrap_or(0.5).clamp(0.25, 0.65) as f32;
            self.tool_split = tool_split::restored(value["tool_split"].as_f64());
            // IL-06: the Items list filter is saved with the layout.
            self.item_list.filter =
                serde_json::from_value(value["item_filter"].clone()).unwrap_or_default();
            for (key, group) in [
                ("upper", &mut self.upper_frame),
                ("lower", &mut self.lower_frame),
            ] {
                if let Ok(saved) = serde_json::from_value::<panel_registry::PanelGroup>(
                    value["panel_groups"][key].clone(),
                ) {
                    group.restore(&saved);
                }
            }
            // A kind saved in both groups stays in the upper one.
            for k in self.upper_frame.tabs.clone() {
                if self.lower_frame.contains(k) && self.lower_frame.tabs.len() > 1 {
                    self.lower_frame.remove(k);
                }
            }
            for (id, key, default, min, max) in [
                (
                    "tools",
                    "left",
                    layout::LEFT_DEFAULT,
                    layout::LEFT_MIN,
                    layout::LEFT_MAX,
                ),
                (
                    "scene",
                    "right",
                    layout::RIGHT_DEFAULT,
                    layout::RIGHT_MIN,
                    layout::RIGHT_MAX,
                ),
            ] {
                let width = value[key]
                    .as_f64()
                    .map_or(default, |w| (w as f32).clamp(min, max));
                ctx.data_mut(|d| {
                    d.insert_persisted(
                        Id::new(id),
                        egui::containers::panel::PanelState {
                            rect: Rect::from_min_size(Pos2::ZERO, vec2(width, 600.)),
                        },
                    )
                });
            }
        }
    }
    /// The side panels in the old layout.json format: File > Project
    /// Presets' Layout preset, and the migration's source (layouts.rs).
    pub(super) fn layout_json(&self, ctx: &Context) -> String {
        let width = |id, default| {
            egui::containers::panel::PanelState::load(ctx, Id::new(id))
                .map_or(default, |s| s.rect.width())
        };
        serde_json::json!({
            "version": 1,
            "split": self.scene_fraction,
            "tool_split": self.tool_split,
            "left": width("tools", layout::LEFT_DEFAULT),
            "right": width("scene", layout::RIGHT_DEFAULT),
            "item_filter": self.item_list.filter,
            // UIC-156: the groups' tabs are saved with the layout.
            "panel_groups": {"upper": self.upper_frame, "lower": self.lower_frame},
        })
        .to_string()
    }
}

/// One Item Transform channel in the units its field shows (degrees for
/// rotation).
fn channel(t: &operandi_core::scene::Transform, prefix: &str, axis: usize) -> f64 {
    match prefix {
        "translation" => t.translation[axis],
        "rotation" => t.rotation[axis].to_degrees(),
        _ => t.scale[axis],
    }
}

/// The unit an Item Transform channel's text carries.
fn channel_unit(prefix: &str) -> &'static str {
    match prefix {
        "translation" => " m",
        "rotation" => "°",
        _ => "",
    }
}
