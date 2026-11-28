//! The Channel Links window (RG-50..53): Driven and Driver columns, the
//! link buttons, a relationship link's keys (RG-52) and the item's user
//! channels (RG-53). Every button runs an `operandi.rig.*` line (UIC-160),
//! so the window, scripts and the MCP bridge make the same edits. A
//! floating window (UIC-183), hidden with the animation switch.
use super::*;
use crate::command_handlers::Commands;
use crate::unavailable::Gate;
use operandi_core::animation::{self, ChannelKey};
use operandi_core::command_registry::{self as table, Area};

#[derive(Default)]
pub(crate) struct ChannelLinks {
    pub(crate) open: bool,
    driven_item: Option<u64>,
    driver_item: Option<u64>,
    driven: Option<String>,
    driver: Option<String>,
    relationship: bool,
    /// The relationship key being edited: the driven value typed.
    driven_value: String,
    /// The Add User Channel row.
    user_name: String,
    user_type: usize,
    user_mode: usize,
}

const TYPES: [&str; 10] = [
    "float",
    "distance",
    "angle",
    "percentage",
    "time",
    "integer",
    "boolean",
    "choice",
    "string",
    "divider",
];
const MODES: [&str; 7] = ["scalar", "xy", "xyz", "rgb", "rgba", "uv", "uvw"];

pub(crate) fn register(r: &mut Commands) {
    r.add_host_command(
        table::script(
            "operandi.rig.links",
            Area::Items,
            "Shows or hides the Channel Links window (RG-50): with two items selected, the first is the driven, the second the driver.",
        )
        .operandi()
        .ui(),
        |a, _| {
            if !crate::anim_switch::enabled() {
                a.status = "Animation is off (OPERANDI_ANIMATION=0 in the environment)".into();
                return;
            }
            a.links.open = !a.links.open;
            if a.links.open {
                a.links_load_selection();
            }
        },
    )
    .unwrap_or_else(|e| panic!("{e}"));
}

impl Operandi {
    /// RG-50: two selected items fill the columns, the first as driven.
    fn links_load_selection(&mut self) {
        let picked = self.picked_items();
        if let Some(&a) = picked.first() {
            self.links.driven_item = Some(a);
        }
        if let Some(&b) = picked.get(1) {
            self.links.driver_item = Some(b);
        }
    }

    /// The selection in pick order (the active item last), as the rig
    /// commands read it.
    fn picked_items(&self) -> Vec<u64> {
        self.editor.scene.picked_items()
    }

    pub(super) fn channel_links_window(&mut self, ctx: &Context) {
        if !self.links.open || !crate::anim_switch::enabled() {
            return;
        }
        const PLACE: &str = "channelLinks";
        let place = self.palettes.places.get(PLACE).copied();
        let size = place.map_or([620., 460.], |p| p.size);
        let spec = crate::floating::OsWindow {
            id: egui::ViewportId::from_hash_of("operandi-channel-links"),
            title: "Channel Links".into(),
            size,
            position: place.and_then(|p| p.position),
            min_size: [420., 300.],
        };
        let shown = self.os_window(ctx, spec, |app, ctx| {
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::side_top_panel(&ctx.style())
                        .fill(ctx.style().visuals.panel_fill)
                        .inner_margin(6.),
                )
                .show(ctx, |ui| app.channel_links_ui(ui));
        });
        let mut place = place.unwrap_or(crate::palettes::Place {
            size,
            position: None,
        });
        if let Some(s) = shown.size
            && s.iter().all(|v| *v >= 100.)
        {
            place.size = s;
        }
        if let Some(at) = shown.position {
            place.position = Some(at);
        }
        self.palettes.places.insert(PLACE.into(), place);
        if !shown.open {
            self.links.open = false;
        }
    }

    fn links_column(&mut self, ui: &mut Ui, driven: bool) {
        let scene = &self.editor.scene;
        let item = if driven {
            self.links.driven_item
        } else {
            self.links.driver_item
        };
        let title = if driven { "Driven" } else { "Driver" };
        let name = item
            .and_then(|i| scene.item(i))
            .map_or("(none)".to_owned(), |i| i.name.clone());
        ui.strong(format!("{title}: {name}"));
        let mut lines = Vec::new();
        let load = if driven { "Load Driven" } else { "Load Driver" };
        if ui
            .button(load)
            .on_hover_text(format!(
                "The selected item becomes the {}",
                title.to_lowercase()
            ))
            .clicked()
        {
            lines.push(());
        }
        if !lines.is_empty() {
            let active = self.editor.scene.selection.item;
            if driven {
                self.links.driven_item = active;
                self.links.driven = None;
            } else {
                self.links.driver_item = active;
                self.links.driver = None;
            }
        }
        let Some(item) = item else {
            return;
        };
        let names = animation::keyable_channels(&self.editor.scene, item);
        let chosen = if driven {
            &mut self.links.driven
        } else {
            &mut self.links.driver
        };
        egui::ScrollArea::vertical()
            .id_salt(("links", driven))
            .max_height(220.)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for n in names {
                    let linked = if driven {
                        operandi_core::rig::driven_by(&self.editor.scene, item, &n).is_some()
                    } else {
                        operandi_core::rig::drives_channel(&self.editor.scene, item, &n)
                    };
                    let label = if linked {
                        format!("{n}  ●")
                    } else {
                        n.clone()
                    };
                    if ui
                        .selectable_label(chosen.as_deref() == Some(n.as_str()), label)
                        .clicked()
                    {
                        *chosen = Some(n);
                    }
                }
            });
    }

    fn channel_links_ui(&mut self, ui: &mut Ui) {
        ui.columns(2, |cols| {
            self.links_column(&mut cols[0], true);
            self.links_column(&mut cols[1], false);
        });
        ui.separator();
        let mut lines: Vec<String> = Vec::new();
        let pair = match (
            self.links.driven_item,
            self.links.driven.as_ref(),
            self.links.driver_item,
            self.links.driver.as_ref(),
        ) {
            (Some(a), Some(ca), Some(b), Some(cb)) => {
                Some((format!("{a}:{ca}"), format!("{b}:{cb}")))
            }
            _ => None,
        };
        let existing = self
            .links
            .driven_item
            .zip(self.links.driven.as_ref())
            .and_then(|(i, c)| {
                operandi_core::rig::driven_by(&self.editor.scene, i, c)
                    .filter(|p| p.kind == "link")
                    .map(|p| p.id)
            });
        ui.horizontal(|ui| {
            if ui
                .button("Swap")
                .on_hover_text("Swap the Driven and Driver columns")
                .clicked()
            {
                std::mem::swap(&mut self.links.driven_item, &mut self.links.driver_item);
                std::mem::swap(&mut self.links.driven, &mut self.links.driver);
            }
            ui.separator();
            ui.selectable_value(&mut self.links.relationship, false, "Direct Link")
                .on_hover_text("The driven channel takes the driver's raw value (RG-51)");
            ui.selectable_value(&mut self.links.relationship, true, "Relationship")
                .on_hover_text("The driven value follows a curve over the driver's value (RG-52)");
            ui.separator();
            let why = if pair.is_none() {
                Some("Pick a channel in each column")
            } else if existing.is_some() {
                Some("That channel is already driven; Remove Link first")
            } else {
                None
            };
            if ui
                .gated(why, egui::Button::new("Add Link"))
                .on_hover_text("operandi.rig.linkAdd")
                .clicked()
                && let Some((to, from)) = &pair
            {
                let mode = if self.links.relationship {
                    "relationship"
                } else {
                    "direct"
                };
                lines.push(format!(
                    "operandi.rig.linkAdd driven:{to} driver:{from} mode:{mode}"
                ));
            }
            if ui
                .gated(
                    existing
                        .is_none()
                        .then_some("The driven channel has no link"),
                    egui::Button::new("Remove Link"),
                )
                .on_hover_text("operandi.rig.linkRemove")
                .clicked()
                && let Some(p) = existing
            {
                lines.push(format!("operandi.rig.linkRemove piece:{p}"));
            }
        });
        // RG-52: the relationship's keys: driver value now, driven typed.
        if let Some(p) = existing
            && let Some(piece) = self.editor.scene.animation.rig.piece(p).cloned()
            && let Some(curve) = &piece.curve
        {
            ui.separator();
            let driver = match (piece.targets.first(), piece.names.get("driver")) {
                (Some(&i), Some(c)) => animation::read(&self.editor.scene, &ChannelKey::new(i, c)),
                _ => None,
            };
            let driver = driver.unwrap_or(0.);
            ui.label(format!("Driver value: {driver:.4}"));
            ui.horizontal(|ui| {
                ui.label("Driven value");
                let field = ui.add(
                    egui::TextEdit::singleline(&mut self.links.driven_value).desired_width(90.),
                );
                crate::numeric_tab::register(ui, &field, Some(&self.links.driven_value));
                let value = self.links.driven_value.trim().parse::<f64>().ok();
                if ui
                    .gated(
                        value.is_none().then_some("Type the driven value"),
                        egui::Button::new("Key"),
                    )
                    .on_hover_text("Store this driver → driven pair\noperandi.rig.relationshipKey")
                    .clicked()
                    && let Some(v) = value
                {
                    lines.push(format!(
                        "operandi.rig.relationshipKey piece:{p} driver:{driver} driven:{v}"
                    ));
                }
                let here = curve
                    .key_at(operandi_core::time_tick::Tick::from_seconds(driver))
                    .is_some();
                if ui
                    .gated(
                        (!here).then_some("No key at the driver's value"),
                        egui::Button::new("Delete Key"),
                    )
                    .on_hover_text("operandi.rig.relationshipKeyRemove")
                    .clicked()
                {
                    lines.push(format!(
                        "operandi.rig.relationshipKeyRemove piece:{p} driver:{driver}"
                    ));
                }
            });
            let keys: Vec<String> = curve
                .keys
                .iter()
                .map(|k| format!("{:.3} → {:.3}", k.time.seconds(), k.value))
                .collect();
            ui.label(if keys.is_empty() {
                "No keys yet".to_owned()
            } else {
                format!("Keys: {}", keys.join(", "))
            });
        }
        // RG-53: the driven item's user channels.
        if let Some(item) = self.links.driven_item {
            ui.separator();
            ui.strong("User Channels");
            let user: Vec<operandi_core::rig::UserChannel> = self
                .editor
                .scene
                .animation
                .rig
                .user
                .iter()
                .filter(|u| u.item == item)
                .cloned()
                .collect();
            for u in &user {
                ui.horizontal(|ui| {
                    ui.label(format!("{} ({}, {})", u.name, u.ty.name(), u.mode));
                    if ui
                        .button("Delete")
                        .on_hover_text("Delete the channel and any links using it\noperandi.rig.userChannelRemove")
                        .clicked()
                    {
                        lines.push(format!("operandi.rig.userChannelRemove item:{item} name:{}", u.name));
                    }
                });
            }
            ui.horizontal(|ui| {
                let field = ui.add(
                    egui::TextEdit::singleline(&mut self.links.user_name)
                        .hint_text("name")
                        .desired_width(110.),
                );
                crate::numeric_tab::register(ui, &field, Some(&self.links.user_name));
                egui::ComboBox::from_id_salt("user-type")
                    .selected_text(TYPES[self.links.user_type])
                    .show_index(ui, &mut self.links.user_type, TYPES.len(), |i| TYPES[i]);
                egui::ComboBox::from_id_salt("user-mode")
                    .selected_text(MODES[self.links.user_mode])
                    .show_index(ui, &mut self.links.user_mode, MODES.len(), |i| MODES[i]);
                let name = self.links.user_name.trim().to_owned();
                if ui
                    .gated(
                        name.is_empty().then_some("Type a name"),
                        egui::Button::new("Add User Channel"),
                    )
                    .on_hover_text("operandi.rig.userChannelAdd")
                    .clicked()
                {
                    lines.push(format!(
                        "operandi.rig.userChannelAdd item:{item} name:{name} type:{} mode:{}",
                        TYPES[self.links.user_type], MODES[self.links.user_mode]
                    ));
                    self.links.user_name.clear();
                }
            });
        }
        for l in lines {
            self.execute(&l);
        }
    }
}
