//! The Display and Assembly Properties tabs every item shares
//! (BEHAVIOR-ITEM-COMMON-TABS Tier 1; settings and rules in
//! operandi_core::item_common), and the item Labels drawn in 3D views.
//!
//! Each control edits every selected item at once (IC-2) as one undo step
//! named after the property (IC-1). Where the selected items disagree the
//! control shows "(mixed)" (UIC-118) until edited.
use super::*;
use crate::unavailable::Gate;
use operandi_core::item_common::{
    Choice, DisplayVisible, ItemCommon, ItemCommonEdit, MAX_LABEL_OFFSET,
};

/// Properties tab names shared by every item kind.
pub(super) const DISPLAY_TAB: &str = "Display";
pub(super) const ASSEMBLY_TAB: &str = "Assembly";

impl Operandi {
    /// The selected items, primary first.
    pub(super) fn common_targets(&self) -> Vec<u64> {
        let s = &self.editor.scene.selection;
        s.item.into_iter().chain(s.also.iter().copied()).collect()
    }

    /// The selected items' shared settings, primary first.
    pub(super) fn common_values(&self) -> Vec<ItemCommon> {
        let ids = self.common_targets();
        ids.iter()
            .filter_map(|id| self.editor.scene.item(*id))
            .map(|i| i.common.clone())
            .collect()
    }

    pub(super) fn run_common(&mut self, edit: ItemCommonEdit) {
        let items = self.common_targets();
        self.run_common_on(items, edit);
    }

    /// Runs `edit` on `items`: through its item.channel line for one item
    /// (UIC-164), so the panel and scripts share the operation. Several
    /// items stay one direct edit (one undo step; item.channel sets one
    /// item, CMD-Q).
    pub(super) fn run_common_on(&mut self, items: Vec<u64>, edit: ItemCommonEdit) {
        if let [item] = items[..]
            && let Some(line) = operandi_core::item_common_channels::common_line(item, &edit)
        {
            #[cfg(test)]
            crate::ui_inventory::note_command(&line);
            self.run_command_line(&line);
            return;
        }
        self.run(Command::EditItemCommon { items, edit });
    }

    /// A Default/Yes/No-style dropdown over every selected item; blank
    /// when they differ.
    pub(super) fn common_choice<T: Copy + PartialEq>(
        &mut self,
        ui: &mut Ui,
        id: &str,
        label: &str,
        values: &[T],
        options: &[(T, &'static str)],
        make: impl Fn(T) -> ItemCommonEdit,
    ) -> Response {
        let first = values.first().copied();
        let same = first.filter(|f| values.iter().all(|v| v == f));
        // UIC-110: filled once any selected item leaves the first option
        // (each list starts at its default); UIC-118: "(mixed)".
        let non_default = same.is_none() || options.first().is_some_and(|(d, _)| same != Some(*d));
        crate::form::rows::status(non_default);
        // UIC-112: a keyable item channel gets its key light.
        if let (Some(v), Some(item)) = (first, self.editor.scene.selection.item) {
            self.anim_row_for(
                &Command::EditItemCommon {
                    items: vec![item],
                    edit: make(v),
                },
                non_default,
            );
        }
        let shown = same
            .and_then(|v| options.iter().find(|(o, _)| *o == v))
            .map_or(crate::form::rows::MIXED, |(_, name)| *name);
        let r = crate::option_row::dropdown(ui, id, label, shown, |ui| {
            let mut picked = None;
            for (option, name) in options {
                if ui.selectable_label(same == Some(*option), *name).clicked() {
                    picked = Some(*option);
                }
            }
            picked
        });
        if let Some(v) = r.inner
            && same != Some(v)
        {
            self.run_common(make(v));
        }
        r.response
    }

    /// Display tab (CP-D01, CP-D06).
    pub(super) fn display_properties(&mut self, ui: &mut Ui) {
        let values = self.common_values();
        let Some(first) = values.first().cloned() else {
            ui.weak("Select an item");
            return;
        };
        let idle = self.editor.live.is_none();
        let locked = self
            .editor
            .scene
            .active()
            .is_some_and(operandi_core::Item::is_locked);
        ui.gated_ui(
            crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE),
            |ui| {
                ui.spacing_mut().item_spacing.y = 2.;
                let visible: Vec<_> = values.iter().map(|c| c.display.visible).collect();
                let options = DisplayVisible::ALL.map(|v| (v, v.label()));
                ui.gated_ui(
                    crate::unavailable::unless(!locked, crate::unavailable::LOCKED),
                    |ui| {
                        self.common_choice(
                            ui,
                            "item-display-visible",
                            "Visible",
                            &visible,
                            &options,
                            ItemCommonEdit::Visible,
                        )
                        .on_hover_text(
                            "How the item is drawn in 3D views (never how it renders): Default \
                     follows the view's Show toggles, Yes always, No never, No (Hide \
                     Children) hides everything under it too",
                        );
                    },
                );
                // Label: one text for all selected items; typed text applies
                // on Enter or when the field loses focus.
                let same_label = values
                    .iter()
                    .all(|c| c.display.label == first.display.label);
                let key = egui::Id::new(("item-label-draft", self.common_targets()));
                let mut text: String = ui.data_mut(|d| {
                    d.get_temp(key).unwrap_or_else(|| {
                        if same_label {
                            first.display.label.clone()
                        } else {
                            String::new()
                        }
                    })
                });
                crate::form::rows::status(!same_label || !first.display.label.is_empty());
                let response = crate::option_row::row(ui, "Label", |ui| {
                    ui.add(
                        TextEdit::singleline(&mut text)
                            .desired_width(ui.available_width())
                            .hint_text(if same_label {
                                ""
                            } else {
                                crate::form::rows::MIXED
                            }),
                    )
                })
                .inner;
                self.text_editors.insert(response.id);
                crate::numeric_tab::register(ui, &response, Some(&text));
                if response.changed() {
                    ui.data_mut(|d| d.insert_temp(key, text.clone()));
                }
                if response.lost_focus() {
                    ui.data_mut(|d| d.remove::<String>(key));
                    let unchanged = same_label && text == first.display.label;
                    if !unchanged && !(text.is_empty() && !same_label) {
                        self.run_common(ItemCommonEdit::Label(text));
                    }
                }
                // CP-D06: typing a Label reveals Show Label and Label Offset.
                if values.iter().any(|c| !c.display.label.is_empty()) {
                    let mut show = first.display.show_label;
                    let mixed = values.iter().any(|c| c.display.show_label != show);
                    let default = ItemCommon::default().display;
                    crate::form::rows::status(show != default.show_label);
                    crate::form::rows::mixed(mixed);
                    if crate::option_row::check(ui, "Show Label", &mut show).changed() {
                        self.run_common(ItemCommonEdit::ShowLabel(show));
                    }
                    for (axis, name) in [(0usize, "Label Offset X"), (1, "Label Offset Y")] {
                        let same = values.iter().all(|c| {
                            c.display.label_offset[axis] == first.display.label_offset[axis]
                        });
                        let mut v = first.display.label_offset[axis];
                        crate::form::rows::status(v != default.label_offset[axis]);
                        let r = crate::option_row::row(ui, name, |ui| {
                            let drag = crate::numeric_tab::DragValue::new(&mut v)
                                .range(-MAX_LABEL_OFFSET..=MAX_LABEL_OFFSET)
                                .speed(1.0)
                                .suffix(" px");
                            crate::form::rows::mixed(!same);
                            ui.add(drag)
                        })
                        .inner;
                        let done = r.drag_stopped()
                            || r.lost_focus()
                            || (r.changed() && !r.dragged() && !r.has_focus());
                        if r.changed() {
                            ui.data_mut(|d| d.insert_temp(r.id, v));
                        }
                        if done {
                            let v = ui.data_mut(|d| d.remove_temp::<f64>(r.id)).unwrap_or(v);
                            if !same || (v - first.display.label_offset[axis]).abs() > 1e-12 {
                                self.run_common(if axis == 0 {
                                    ItemCommonEdit::LabelOffsetX(v)
                                } else {
                                    ItemCommonEdit::LabelOffsetY(v)
                                });
                            }
                        }
                    }
                }
            },
        );
    }

    /// Assembly tab (CP-A01, CP-A02).
    pub(super) fn assembly_properties(&mut self, ui: &mut Ui) {
        let values = self.common_values();
        if values.is_empty() {
            ui.weak("Select an item");
            return;
        }
        let idle = self.editor.live.is_none();
        ui.gated_ui(
            crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE),
            |ui| {
                ui.spacing_mut().item_spacing.y = 2.;
                let options = Choice::ALL.map(|v| (v, v.label()));
                let select: Vec<_> = values.iter().map(|c| c.assembly.select).collect();
                self.common_choice(
                    ui,
                    "item-assembly-select",
                    "Select",
                    &select,
                    &options,
                    ItemCommonEdit::Select,
                )
                .on_hover_text(
                    "No: can't be picked in 3D views; still selectable in the Items list",
                );
                let lock: Vec<_> = values.iter().map(|c| c.assembly.lock).collect();
                self.common_choice(
                ui,
                "item-assembly-lock",
                "Lock",
                &lock,
                &options,
                ItemCommonEdit::Lock,
            )
            .on_hover_text(
                "Yes: the item can't be moved, rotated or scaled, and its Visible row is locked",
            );
            },
        );
    }

    /// CP-D06: every drawn item's Label next to its centre, offset by Label
    /// Offset (points, Y up). Selected items use the selection colour.
    pub(super) fn draw_item_labels(&self, painter: &egui::Painter, r: Rect) {
        let scene = &self.editor.scene;
        if scene.items.iter().all(|i| i.shown_label().is_none()) {
            return;
        }
        let drawing = &self.display.settings.drawing;
        let projector = self.camera.projector(r);
        for item in &scene.items {
            let Some(text) = item.shown_label() else {
                continue;
            };
            let kind_shown = if item.camera.is_some() {
                drawing.cameras
            } else if item.light.is_some() {
                drawing.lights
            } else if item.locator.is_some() {
                drawing.locators
            } else {
                true
            };
            if !scene.drawn_in_viewport(item.id, kind_shown) {
                continue;
            }
            let Ok(world) = scene.world_matrix(item.id) else {
                continue;
            };
            let Some((at, _)) = projector.project(world.transform_point3(DVec3::ZERO)) else {
                continue;
            };
            let [dx, dy] = item.common.display.label_offset;
            let at = at + vec2(dx as f32, -(dy as f32)) + vec2(6., -6.);
            if !r.contains(at) {
                continue;
            }
            let selected =
                scene.selection.item == Some(item.id) || scene.selection.also.contains(&item.id);
            let color = if selected {
                theme::viewport::selection()
            } else {
                theme::viewport::locator()
            };
            painter.text(
                at,
                Align2::LEFT_BOTTOM,
                text,
                FontId::proportional(12.),
                color,
            );
        }
    }
}
