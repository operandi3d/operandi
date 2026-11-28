//! The shared item tabs' Tier 3 rows (operandi_core::item_extras): more of
//! the Display and Assembly tabs, the User Channels and Tags tabs, and what
//! they do in 3D views (Pivot / Center markers, Dimensions, Notes, Tooltip,
//! Draw Options on camera and light glyphs, Redirect, Pick Walk).
//!
//! Every row edits all selected items at once as one undo step (IC-1/2);
//! rows show the primary item's value.
use super::*;
use crate::unavailable::Gate;
use operandi_core::drop_action::DropAction;
use operandi_core::item_common::{Choice, ItemCommonEdit};
use operandi_core::item_extras::*;

pub(super) const CHANNELS_TAB: &str = "User Channels";
pub(super) const TAGS_TAB: &str = "Tags";

impl Operandi {
    fn run_extra(&mut self, edit: ExtraEdit) {
        self.run_common(ItemCommonEdit::Extra(edit));
    }

    fn primary_extras(&self) -> Option<(operandi_core::Item, ItemExtras)> {
        let item = self.editor.scene.active()?.clone();
        let x = item.common.extras.clone();
        Some((item, x))
    }

    /// Before an extras row: status circle and "(mixed)" (UIC-110/118),
    /// by trying `edit` (the row at the primary's value) on a fresh item
    /// and on the other selected items. Returns whether they differ.
    fn probe_extra_row(&self, edit: ExtraEdit) -> bool {
        use operandi_core::item_common::ItemCommon;
        let edit = ItemCommonEdit::Extra(edit);
        let changes = |c: &ItemCommon| {
            let mut x = c.clone();
            x.apply(&edit).is_ok() && x != *c
        };
        let values = self.common_values();
        let mixed = values.iter().skip(1).any(changes);
        crate::form::rows::status(changes(&ItemCommon::default()));
        // UIC-112: a keyable item channel gets its key light.
        if let Some(item) = self.editor.scene.selection.item {
            self.anim_row_for(
                &Command::EditItemCommon {
                    items: vec![item],
                    edit: edit.clone(),
                },
                changes(&ItemCommon::default()),
            );
        }
        mixed
    }

    /// A one-line text row that applies when the field loses focus.
    fn extra_text_row(
        &mut self,
        ui: &mut Ui,
        key: &str,
        label: &str,
        value: &str,
        multiline: bool,
        make: impl Fn(String) -> ExtraEdit,
    ) {
        let id = egui::Id::new(("extra-text", key, self.common_targets()));
        let mixed = self.probe_extra_row(make(value.to_string()));
        // UIC-118: "(mixed)" as the empty field's hint until typed.
        let shown = if mixed { "" } else { value };
        let mut text: String = ui.data_mut(|d| d.get_temp(id).unwrap_or_else(|| shown.to_string()));
        let hint = if mixed { crate::form::rows::MIXED } else { "" };
        let r = crate::option_row::row(ui, label, |ui| {
            if multiline {
                ui.add(
                    TextEdit::multiline(&mut text)
                        .desired_rows(3)
                        .hint_text(hint)
                        .desired_width(ui.available_width()),
                )
            } else {
                ui.add(
                    TextEdit::singleline(&mut text)
                        .hint_text(hint)
                        .desired_width(ui.available_width()),
                )
            }
        })
        .inner;
        self.text_editors.insert(r.id);
        crate::numeric_tab::register(ui, &r, Some(&text));
        if r.changed() {
            ui.data_mut(|d| d.insert_temp(id, text.clone()));
        }
        if r.lost_focus() {
            ui.data_mut(|d| d.remove::<String>(id));
            if crate::numeric_tab::accepted(&r) && text != shown {
                self.run_extra(make(text));
            }
        }
    }

    /// An item-picker dropdown (with "(none)") over the scene's items,
    /// leaving out the selected ones.
    fn item_picker(
        &mut self,
        ui: &mut Ui,
        key: &str,
        label: &str,
        value: Option<u64>,
        make: impl Fn(Option<u64>) -> ExtraEdit,
    ) {
        let mixed = self.probe_extra_row(make(value));
        let targets = self.common_targets();
        let items: Vec<(u64, String)> = self
            .editor
            .scene
            .items
            .iter()
            .filter(|i| !targets.contains(&i.id))
            .map(|i| (i.id, i.name.clone()))
            .collect();
        let shown = if mixed {
            crate::form::rows::MIXED.to_string()
        } else {
            value.map_or("(none)".to_string(), |v| {
                items
                    .iter()
                    .find(|i| i.0 == v)
                    .map_or("(missing)".to_string(), |i| i.1.clone())
            })
        };
        let picked =
            crate::option_row::dropdown(ui, &format!("{key}-{targets:?}"), label, &shown, |ui| {
                let mut picked = None;
                if ui.selectable_label(value.is_none(), "(none)").clicked() {
                    picked = Some(None);
                }
                for (id, name) in &items {
                    if ui.selectable_label(value == Some(*id), name).clicked() {
                        picked = Some(Some(*id));
                    }
                }
                picked
            })
            .inner;
        if let Some(p) = picked
            && (mixed || p != value)
        {
            self.run_extra(make(p));
        }
    }

    /// Display tab rows after Label: Pivot, Center, then the Add / Remove
    /// gadgets (CP-D02/03, D07..09).
    pub(super) fn display_extra_rows(&mut self, ui: &mut Ui) {
        let Some((item, x)) = self.primary_extras() else {
            return;
        };
        ui.gated_ui(None, |ui| {
            ui.spacing_mut().item_spacing.y = 2.;
            let options = Choice::ALL.map(|v| (v, v.label()));
            ui.gated_ui(crate::unavailable::unless(!item.is_locked(), crate::unavailable::LOCKED), |ui| {
                let mut pivot = x.pivot;
                crate::option_row::choice(ui, "item-pivot", "Pivot", &mut pivot, &options)
                    .response
                    .on_hover_text("Yes: draw the item's pivot marker in 3D views");
                if pivot != x.pivot {
                    self.run_extra(ExtraEdit::Pivot(pivot));
                }
                let mut center = x.center;
                crate::option_row::choice(ui, "item-center", "Center", &mut center, &options)
                    .response
                    .on_hover_text("Yes: draw the item's centre marker in 3D views");
                if center != x.center {
                    self.run_extra(ExtraEdit::Center(center));
                }
            });
            ui.separator();
            // CP-D07 Draw Options.
            let toggle = |ui: &mut Ui, on: bool, add: &str, remove: &str| {
                ui.horizontal(|ui| {
                    crate::option_row::label(ui, "");
                    ui.button(if on { remove } else { add }).clicked()
                })
                .inner
            };
            if toggle(ui, x.draw_options.is_some(), "Add Draw Options", "Remove Draw Options") {
                self.run_extra(ExtraEdit::DrawOptions(x.draw_options.is_none()));
            }
            if let Some(d) = &x.draw_options {
                let mut enable = d.enable;
                if crate::option_row::check(ui, "Enable", &mut enable).changed() {
                    self.run_extra(ExtraEdit::DrawEnable(enable));
                }
                let wires = WireColor::ALL.map(|w| (w, w.label()));
                let mut w = d.wireframe;
                crate::option_row::choice(ui, "item-wireframe", "Wireframe", &mut w, &wires)
                    .response
                    .on_hover_text(
                        "Camera and light glyphs use it now; meshes keep the view's wire colour (provisional)",
                    );
                if w != d.wireframe {
                    self.run_extra(ExtraEdit::Wireframe(w));
                }
                ui.gated_ui(crate::unavailable::unless(d.wireframe == WireColor::User, "Set Wireframe to User first"), |ui| {
                    ui.horizontal(|ui| {
                        crate::option_row::label(ui, "Wireframe Color");
                        crate::option_row::fill(ui, |ui| {
                            let mut rgb = d.wire_color;
                            if crate::colour_field::show(
                                ui,
                                &mut rgb,
                                crate::colour_field::Options::ui(),
                            )
                            .changed()
                            {
                                self.run_extra(ExtraEdit::WireColor(rgb));
                            }
                        })
                    });
                });
            }
            ui.separator();
            // CP-D08 Dimensions (meshes).
            if item.is_mesh() {
                if toggle(ui, x.dimensions.is_some(), "Add Dimensions", "Remove Dimensions") {
                    self.run_extra(ExtraEdit::Dimensions(x.dimensions.is_none()));
                }
                if let Some(d) = &x.dimensions {
                    let shows = DimensionsShow::ALL.map(|s| (s, s.label()));
                    let mut v = d.visibility;
                    crate::option_row::choice(ui, "item-dims-show", "Visibility", &mut v, &shows);
                    if v != d.visibility {
                        self.run_extra(ExtraEdit::DimensionsShow(v));
                    }
                    let mut opacity = d.opacity * 100.0;
                    let r = crate::option_row::row(ui, "Opacity", |ui| {
                        ui.add(crate::numeric_tab::DragValue::new(&mut opacity).range(0.0..=100.0).suffix(" %"))
                    })
                    .inner;
                    if (r.drag_stopped() || r.lost_focus()) && (opacity / 100.0 - d.opacity).abs() > 1e-9 {
                        self.run_extra(ExtraEdit::DimensionsOpacity(opacity / 100.0));
                    }
                    for (axis, name) in ["Show X", "Show Y", "Show Z"].iter().enumerate() {
                        let mut on = d.show[axis];
                        if crate::option_row::check(ui, name, &mut on).changed() {
                            self.run_extra(ExtraEdit::DimensionsAxis(axis, on));
                        }
                    }
                }
            }
            // CP-D09 Notes.
            if toggle(ui, x.notes.is_some(), "Add Notes", "Remove Notes") {
                self.run_extra(ExtraEdit::Notes(x.notes.is_none()));
            }
            if let Some(n) = &x.notes {
                self.extra_text_row(ui, "notes-title", "Title", &n.title, false, ExtraEdit::NotesTitle);
                for i in 0..NOTE_LINES {
                    let line = n.lines.get(i).cloned().unwrap_or_default();
                    let key = format!("note-{i}");
                    self.extra_text_row(ui, &key, &format!("Line {}", i + 1), &line, false, move |t| {
                        ExtraEdit::NoteLine(i, t)
                    });
                }
            }
        });
    }

    /// Assembly tab rows after Select and Lock (CP-A03, A04, A06, A08,
    /// A09).
    pub(super) fn assembly_extra_rows(&mut self, ui: &mut Ui) {
        let Some((_, x)) = self.primary_extras() else {
            return;
        };
        ui.gated_ui(None, |ui| {
            ui.spacing_mut().item_spacing.y = 2.;
            self.item_picker(ui, "item-redirect", "Redirect", x.redirect, ExtraEdit::Redirect);
            for walk in Walk::ALL {
                self.item_picker(ui, walk.label(), walk.label(), x.pick_walk[walk as usize], move |v| {
                    ExtraEdit::PickWalk(walk, v)
                });
            }
            ui.separator();
            self.extra_text_row(ui, "tooltip", "Tooltip", &x.tooltip, false, ExtraEdit::Tooltip);
            self.extra_text_row(ui, "user-tag", "User Tag", &x.user_tag, false, ExtraEdit::UserTag);
            ui.separator();
            for (slot, name) in DROP_SLOTS.iter().enumerate() {
                let value = x.drop_actions[slot];
                let shown = value.map_or("(none)", |a| a.label());
                let picked = crate::option_row::dropdown(
                    ui,
                    &format!("item-drop-{slot}"),
                    &format!("Drop Action: {name}"),
                    shown,
                    |ui| {
                        let mut picked = None;
                        if ui.selectable_label(value.is_none(), "(none)").clicked() {
                            picked = Some(None);
                        }
                        for a in DropAction::ALL.into_iter().filter(|a| *a != DropAction::None) {
                            if ui.selectable_label(value == Some(a), a.label()).clicked() {
                                picked = Some(Some(a));
                            }
                        }
                        picked
                    },
                )
                .inner;
                if let Some(p) = picked
                    && p != value
                {
                    self.run_extra(ExtraEdit::DropAction(slot, p));
                }
            }
            ui.weak("Source and Destination override the top bar's Drop Action; Assembly is stored (no presets yet)");
        });
    }

    /// CP-U01..04: the User Channels tab.
    pub(super) fn user_channels_tab(&mut self, ui: &mut Ui) {
        let Some((_, x)) = self.primary_extras() else {
            ui.weak("Select an item");
            return;
        };
        // RG-53 rig user channels (imported rig controls) first.
        if self.rig_user_channel_rows(ui) {
            ui.separator();
        }
        ui.gated_ui(None, |ui| {
            ui.spacing_mut().item_spacing.y = 2.;
            for ch in &x.channels {
                let name = ch.name.clone();
                if ch.kind == ChannelType::Divider {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.strong(ch.label());
                        if ui.small_button("Remove").clicked() {
                            self.run_extra(ExtraEdit::RemoveChannel(name.clone()));
                        }
                    });
                    continue;
                }
                ui.horizontal(|ui| {
                    crate::option_row::label(ui, ch.label()).on_hover_text(format!(
                        "{} ({})",
                        ch.name,
                        ch.kind.label()
                    ));
                    match (&ch.value, ch.kind) {
                        (ChannelValue::Bool(b), _) => {
                            let mut on = *b;
                            if ui.checkbox(&mut on, "").changed() {
                                self.run_extra(ExtraEdit::ChannelValue(
                                    name.clone(),
                                    ChannelValue::Bool(on),
                                ));
                            }
                        }
                        (ChannelValue::Text(t), _) => {
                            let key = egui::Id::new(("channel-text", &name, self.common_targets()));
                            let mut text: String =
                                ui.data_mut(|d| d.get_temp(key).unwrap_or_else(|| t.clone()));
                            let r = ui.add(TextEdit::singleline(&mut text).desired_width(120.));
                            self.text_editors.insert(r.id);
                            crate::numeric_tab::register(ui, &r, Some(&text));
                            if r.changed() {
                                ui.data_mut(|d| d.insert_temp(key, text.clone()));
                            }
                            if r.lost_focus() {
                                ui.data_mut(|d| d.remove::<String>(key));
                                if crate::numeric_tab::accepted(&r) && &text != t {
                                    self.run_extra(ExtraEdit::ChannelValue(
                                        name.clone(),
                                        ChannelValue::Text(text),
                                    ));
                                }
                            }
                        }
                        (ChannelValue::Number(n), kind) => {
                            let (scale, suffix) = match kind {
                                ChannelType::Distance => (1.0, " m"),
                                ChannelType::Angle => (180.0 / std::f64::consts::PI, "°"),
                                ChannelType::Percent => (100.0, " %"),
                                _ => (1.0, ""),
                            };
                            let key = egui::Id::new(("channel-num", &name, self.common_targets()));
                            let mut shown: f64 =
                                ui.data_mut(|d| d.get_temp(key).unwrap_or(n * scale));
                            let mut drag = crate::numeric_tab::DragValue::new(&mut shown)
                                .speed(0.1)
                                .suffix(suffix);
                            if kind == ChannelType::Integer {
                                drag = drag.fixed_decimals(0);
                            }
                            let r = ui.add(drag);
                            if r.changed() {
                                ui.data_mut(|d| d.insert_temp(key, shown));
                            }
                            if r.drag_stopped()
                                || r.lost_focus()
                                || (r.changed() && !r.dragged() && !r.has_focus())
                            {
                                ui.data_mut(|d| d.remove::<f64>(key));
                                if (shown / scale - n).abs() > 1e-12 {
                                    self.run_extra(ExtraEdit::ChannelValue(
                                        name.clone(),
                                        ChannelValue::Number(shown / scale),
                                    ));
                                }
                            }
                        }
                        _ => {}
                    }
                    if ui
                        .small_button("✕")
                        .on_hover_text("Remove this channel")
                        .clicked()
                    {
                        self.run_extra(ExtraEdit::RemoveChannel(name.clone()));
                    }
                });
            }
            ui.add_space(4.);
            self.add_user_channel_button(ui);
            ui.weak("User channels are stored with the item; nothing links them yet");
        });
    }

    /// CP-U02: Add User Channel... with Username, Channel Name, Type,
    /// Default Value and optional limits.
    fn add_user_channel_button(&mut self, ui: &mut Ui) {
        #[derive(Clone, Default)]
        struct Draft {
            name: String,
            username: String,
            kind: ChannelType,
            number: f64,
            text: String,
            boolean: bool,
            use_min: bool,
            min: f64,
            use_max: bool,
            max: f64,
        }
        let key = egui::Id::new("add-user-channel-draft");
        ui.menu_button("Add User Channel...", |ui| {
            let mut d: Draft = ui.data_mut(|m| m.get_temp(key).unwrap_or_default());
            let r = ui.add(TextEdit::singleline(&mut d.username).hint_text("Username (optional)"));
            self.text_editors.insert(r.id);
            crate::numeric_tab::register(ui, &r, Some(&d.username));
            let r = ui.add(TextEdit::singleline(&mut d.name).hint_text("Channel Name"));
            self.text_editors.insert(r.id);
            crate::numeric_tab::register(ui, &r, Some(&d.name));
            egui::ComboBox::from_id_salt("add-channel-type")
                .selected_text(d.kind.label())
                .show_ui(ui, |ui| {
                    for k in ChannelType::ALL {
                        ui.selectable_value(&mut d.kind, k, k.label());
                    }
                });
            match d.kind {
                ChannelType::Boolean => {
                    ui.checkbox(&mut d.boolean, "Default Value");
                }
                ChannelType::String => {
                    let r = ui.add(TextEdit::singleline(&mut d.text).hint_text("Default Value"));
                    self.text_editors.insert(r.id);
                    crate::numeric_tab::register(ui, &r, Some(&d.text));
                }
                ChannelType::Divider => {}
                _ => {
                    ui.horizontal(|ui| {
                        ui.label("Default Value");
                        ui.add(crate::numeric_tab::DragValue::new(&mut d.number).speed(0.1));
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut d.use_min, "Use Minimum Value");
                        ui.gated(
                            crate::unavailable::unless(d.use_min, "Tick Use Minimum Value first"),
                            crate::numeric_tab::DragValue::new(&mut d.min).speed(0.1),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut d.use_max, "Use Maximum Value");
                        ui.gated(
                            crate::unavailable::unless(d.use_max, "Tick Use Maximum Value first"),
                            crate::numeric_tab::DragValue::new(&mut d.max).speed(0.1),
                        );
                    });
                }
            }
            let value = match d.kind {
                ChannelType::Boolean => ChannelValue::Bool(d.boolean),
                ChannelType::String => ChannelValue::Text(d.text.clone()),
                ChannelType::Divider => ChannelValue::None,
                // Shown units: percent and degrees typed as shown.
                ChannelType::Percent => ChannelValue::Number(d.number / 100.0),
                ChannelType::Angle => ChannelValue::Number(d.number.to_radians()),
                _ => ChannelValue::Number(d.number),
            };
            let channel = UserChannel::new(
                &d.name,
                &d.username,
                d.kind,
                value,
                d.use_min.then_some(d.min),
                d.use_max.then_some(d.max),
            );
            ui.data_mut(|m| m.insert_temp(key, d.clone()));
            match channel {
                Ok(ch) => {
                    if ui.button("Add").clicked() {
                        self.run_extra(ExtraEdit::AddChannel(ch));
                        ui.data_mut(|m| m.remove::<Draft>(key));
                        ui.close();
                    }
                }
                Err(e) => {
                    ui.gated(Some(&*e), Button::new("Add"));
                }
            }
        });
    }

    /// CP-T01..03: the Tags tab.
    pub(super) fn tags_tab(&mut self, ui: &mut Ui) {
        let Some((_, x)) = self.primary_extras() else {
            ui.weak("Select an item");
            return;
        };
        ui.gated_ui(None, |ui| {
            ui.spacing_mut().item_spacing.y = 2.;
            for tag in &x.tags {
                let kind = tag.kind.clone();
                let multiline = kind == COMMENT_TAG;
                self.extra_text_row(
                    ui,
                    &format!("tag-{kind}"),
                    &tag.kind,
                    &tag.value,
                    multiline,
                    move |t| ExtraEdit::SetTag(kind.clone(), t),
                );
            }
            ui.horizontal(|ui| {
                let custom = egui::Id::new("custom-tag-name");
                ui.menu_button("Add Tags", |ui| {
                    if x.tags.iter().all(|t| t.kind != COMMENT_TAG)
                        && ui.button(COMMENT_TAG).clicked()
                    {
                        self.run_extra(ExtraEdit::SetTag(COMMENT_TAG.into(), String::new()));
                        ui.close();
                    }
                    ui.separator();
                    let mut name: String = ui.data_mut(|d| d.get_temp(custom).unwrap_or_default());
                    let r = ui.add(TextEdit::singleline(&mut name).hint_text("Custom tag type"));
                    self.text_editors.insert(r.id);
                    crate::numeric_tab::register(ui, &r, Some(&name));
                    ui.data_mut(|d| d.insert_temp(custom, name.clone()));
                    let ok =
                        !name.trim().is_empty() && x.tags.iter().all(|t| t.kind != name.trim());
                    if ui
                        .gated(
                            crate::unavailable::unless(
                                ok,
                                if name.trim().is_empty() {
                                    "Type a tag type first"
                                } else {
                                    "The item already has that tag"
                                },
                            ),
                            Button::new("Add Custom Tag"),
                        )
                        .clicked()
                    {
                        self.run_extra(ExtraEdit::SetTag(name.trim().into(), String::new()));
                        ui.data_mut(|d| d.remove::<String>(custom));
                        ui.close();
                    }
                });
                ui.gated_ui(
                    crate::unavailable::unless(
                        !x.tags.is_empty(),
                        "The item has no tags to remove",
                    ),
                    |ui| {
                        ui.menu_button("Remove Tags", |ui| {
                            for tag in &x.tags {
                                if ui.button(&tag.kind).clicked() {
                                    self.run_extra(ExtraEdit::RemoveTag(tag.kind.clone()));
                                    ui.close();
                                }
                            }
                        });
                    },
                );
            });
        });
    }

    /// In 3D views: Pivot and Center markers, Dimensions and Notes of the
    /// items that have them, and the hovered item's Tooltip.
    pub(super) fn draw_item_extras(&self, painter: &egui::Painter, r: Rect) {
        let scene = &self.editor.scene;
        if scene
            .items
            .iter()
            .all(|i| i.common.extras == ItemExtras::default())
        {
            return;
        }
        let projector = self.camera.projector(r);
        let font = FontId::proportional(11.);
        let hidden = scene.display_hidden();
        for item in scene
            .items
            .iter()
            .filter(|i| i.visible && !hidden.contains(&i.id))
        {
            let x = &item.common.extras;
            if *x == ItemExtras::default() {
                continue;
            }
            let selected =
                scene.selection.item == Some(item.id) || scene.selection.also.contains(&item.id);
            let Ok(world) = scene.world_matrix(item.id) else {
                continue;
            };
            let origin = projector
                .project(world.transform_point3(DVec3::ZERO))
                .map(|p| p.0);
            let color = theme::viewport::locator();
            if let Some(o) = origin {
                // CP-D03 Center: a small cross; CP-D02 Pivot: a ring.
                if x.center == Choice::Yes {
                    let s = Stroke::new(1.2f32, color);
                    painter.line_segment([o - vec2(5., 0.), o + vec2(5., 0.)], s);
                    painter.line_segment([o - vec2(0., 5.), o + vec2(0., 5.)], s);
                }
                if x.pivot == Choice::Yes {
                    painter.circle_stroke(o, 4., Stroke::new(1.2f32, color));
                }
                // CP-D09 Notes, while selected.
                if selected && let Some(n) = &x.notes {
                    let mut text = n.title.clone();
                    for l in n.lines.iter().filter(|l| !l.is_empty()) {
                        text.push('\n');
                        text.push_str(l);
                    }
                    if !text.trim().is_empty() {
                        painter.text(
                            o + vec2(12., 12.),
                            Align2::LEFT_TOP,
                            text.trim(),
                            font.clone(),
                            color,
                        );
                    }
                }
            }
            // CP-D08 Dimensions: the world bounding box's size per axis.
            if let Some(d) = &x.dimensions
                && d.visibility != DimensionsShow::Off
                && (d.visibility == DimensionsShow::On || selected)
                && !item.mesh.vertices.is_empty()
            {
                let mut lo = DVec3::splat(f64::INFINITY);
                let mut hi = DVec3::splat(f64::NEG_INFINITY);
                for v in &item.mesh.vertices {
                    let p = world.transform_point3(*v);
                    lo = lo.min(p);
                    hi = hi.max(p);
                }
                let alpha = (d.opacity.clamp(0.0, 1.0) * 255.0) as u8;
                let c = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);
                let size = hi - lo;
                for (axis, name) in ["X", "Y", "Z"].iter().enumerate() {
                    if !d.show[axis] {
                        continue;
                    }
                    let mut a = lo;
                    let mut b = lo;
                    b[axis] = hi[axis];
                    if let (Some((pa, _)), Some((pb, _))) =
                        (projector.project(a), projector.project(b))
                    {
                        painter.extend(egui::Shape::dashed_line(
                            &[pa, pb],
                            Stroke::new(1.0f32, c),
                            4.0,
                            3.0,
                        ));
                        a[axis] = (lo[axis] + hi[axis]) / 2.0;
                        if let Some((mid, _)) = projector.project(a) {
                            painter.text(
                                mid,
                                Align2::CENTER_BOTTOM,
                                format!("{name} {:.3} m", size[axis]),
                                font.clone(),
                                c,
                            );
                        }
                    }
                }
            }
        }
    }

    /// CP-A06: the Tooltip of the item under the pointer (Item mode).
    pub(super) fn item_tooltip(&self, pos: Pos2, r: Rect) -> Option<String> {
        if self.editor.scene.selection.mode != Mode::Item {
            return None;
        }
        let (id, _) = self.pick_component(pos, r)?;
        let item = self.editor.scene.item(id)?;
        let t = item.common.extras.tooltip.trim();
        (!t.is_empty()).then(|| t.to_string())
    }

    /// CP-A04: Pick Walk with the arrow keys in Item mode, no tool active.
    /// Returns true when it selected an item.
    pub(super) fn pick_walk_key(&mut self, ctx: &egui::Context) -> bool {
        // Plain arrows only: Shift/Ctrl+arrows are the animation keys
        // (TK-63; egui's consume_key would also take them with Shift).
        if self.editor.scene.selection.mode != Mode::Item
            || self.editor.session.live.is_some()
            || ctx.input(|i| crate::anim_keys::owns_arrow(i.modifiers))
        {
            return false;
        }
        let Some(from) = self.editor.scene.selection.item else {
            return false;
        };
        let keys = [
            (Key::ArrowUp, Walk::Up),
            (Key::ArrowDown, Walk::Down),
            (Key::ArrowLeft, Walk::Left),
            (Key::ArrowRight, Walk::Right),
        ];
        for (key, walk) in keys {
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, key)) {
                if let Some(to) = self.editor.scene.pick_walk(from, walk) {
                    self.run(Command::SelectItem(Some(to)));
                }
                return true;
            }
        }
        false
    }
}

/// CP-D07 for camera and light glyphs: the colour Draw Options gives
/// `item`'s lines, or `None` for the view's usual colour.
pub(super) fn draw_options_color(item: &operandi_core::Item) -> Option<Color32> {
    let d = item
        .common
        .extras
        .draw_options
        .as_ref()
        .filter(|d| d.enable)?;
    match d.wireframe {
        WireColor::Default => None,
        WireColor::User => {
            let [r, g, b] = d.wire_color.map(|c| (c.clamp(0.0, 1.0) * 255.0) as u8);
            Some(Color32::from_rgb(r, g, b))
        }
        WireColor::BackgroundLight => Some(theme::viewport::locator()),
        WireColor::BackgroundDark => Some(theme::viewport::colored_wire()),
    }
}
