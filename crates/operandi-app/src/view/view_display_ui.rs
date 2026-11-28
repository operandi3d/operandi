//! The view header's Display menu and the viewport preset dialogs
//! (view_display.rs holds the settings and their rules).
use super::*;
use crate::unavailable::Gate;
use view_display::{Group, Include, ShowVertices, Style};

/// Save Viewport Preset… or Delete Preset…, in the shared form dialog
/// (UIC-80..99).
pub(super) struct PresetDialog {
    save: bool,
    /// Delete: the saved presets listed when it opened.
    names: Vec<String>,
    form: form::Dialog,
}
fn save_rows() -> Vec<form::Row> {
    use form::Row;
    vec![
        Row::text("name", "Name", "")
            .fresh()
            .tip("The preset's name; an existing name is replaced"),
        Row::check("drawing", "Drawing", true).tip("Include the drawing toggles and overrides"),
        Row::check("active", "Active Meshes", true).tip("Include the active meshes' look"),
        Row::check("inactive", "Inactive Meshes", true).tip("Include the inactive meshes' look"),
    ]
}

impl Operandi {
    /// Save Viewport Preset… (the viewport menu and Viewport Properties'
    /// Presets menu).
    pub(super) fn open_save_viewport_preset(&mut self) {
        self.preset_dialog = Some(PresetDialog {
            save: true,
            names: Vec::new(),
            form: self
                .form_dialog("Save Viewport Preset", save_rows())
                .focus("name"),
        });
    }
    /// The look the view draws an item with (its group, with the view's
    /// overrides); `None` when the view hides it.
    #[cfg(test)]
    pub(super) fn item_look(&self, id: u64) -> Option<Group> {
        self.item_look_as(id, self.editor.scene.is_active_mesh(id))
    }
    /// `item_look` with the item's activeness already known
    /// (`Scene::active_item_set`), for loops over every item.
    pub(super) fn item_look_as(&self, id: u64, active: bool) -> Option<Group> {
        let selection = &self.editor.scene.selection;
        if self
            .editor
            .scene
            .item(id)
            .is_none_or(|i| self.show_meshes_hides(i))
        {
            return None;
        }
        self.display
            // VP-04/05: every selected mesh item is active, whatever the
            // selection mode; with no mesh selected, the last selected one
            // stays active (last_mesh.rs).
            .group(active)
            .map(|g| self.display.effective(g, selection.mode))
    }

    /// Show Meshes off: no mesh item draws (cameras, lights and backdrops
    /// have their own switches). Both drawing paths ask (#57: the
    /// graphics-card path didn't, so the switch did nothing there).
    pub(super) fn show_meshes_hides(&self, item: &operandi_core::Item) -> bool {
        !self.display.settings.drawing.meshes
            && item.camera.is_none()
            && item.light.is_none()
            && item.backdrop.is_none()
    }

    /// An item's plain wire colour on the CPU drawing paths, as the GPU
    /// path's Look picks it (ViewDisplay::plain_wire_color), else the
    /// theme's Wireframe.
    pub(super) fn plain_wire_as(&self, active: bool) -> Color32 {
        self.display
            .plain_wire_color(active)
            .unwrap_or_else(theme::viewport::wireframe)
    }

    /// The view's active meshes draw as Wireframe (wider wire, lasso
    /// selects through).
    pub(super) fn wireframe_view(&self) -> bool {
        self.display.settings.active.style == Style::Wireframe
    }

    /// Display: styles (the active meshes'), toggles, then presets.
    /// Returns the Display button's rect.
    pub(super) fn display_menu(&mut self, ui: &mut Ui) -> Rect {
        let presets = self.view_presets.all();
        // The button names this view's active-mesh style and is as wide as
        // the longest style name, so it doesn't jump when the style changes.
        let font = egui::TextStyle::Button.resolve(ui.style());
        let widest = Style::ALL
            .iter()
            .map(|s| {
                ui.fonts_mut(|f| {
                    f.layout_no_wrap(s.short_label().into(), font.clone(), Color32::WHITE)
                        .size()
                        .x
                })
            })
            .fold(0., f32::max);
        let padding = ui.spacing().button_padding.x * 2.;
        let button = Button::new(self.display.menu_label()).min_size(vec2(widest + padding, 0.));
        let tip = format!(
            "Shading style for this view's active meshes: {} (inactive meshes: Viewport Properties)",
            self.display.settings.active.style.label()
        );
        let response = egui::containers::menu::MenuButton::from_button(button).ui(ui, |ui| {
            ui.set_min_width(240.);
            // VP-30..34: only the display styles, no active/inactive split;
            // a pick sets Active Meshes > Shading Style for this view (the
            // inactive look is set only in Viewport Properties).
            let active = self.display.settings.active.style;
            for style in Style::ALL {
                let text = format!(
                    "{} {}",
                    if style == active { "✔" } else { "   " },
                    style.label()
                );
                if ui
                    .add(Button::new(text).frame(false))
                    .on_hover_text("This view's active meshes")
                    .clicked()
                {
                    self.display.set_style(style, false);
                    self.status = format!("Shading style: {}", style.label());
                    ui.close();
                }
            }
            let d = &mut self.display.settings.drawing;
            ui.separator();
            ui.checkbox(&mut d.grid, "Show Grid");
            ui.checkbox(&mut d.backdrops, "Show Backdrop")
                .on_hover_text("Off hides every Backdrop in this view only");
            ui.checkbox(&mut d.cameras, "Show Cameras")
                .on_hover_text("Draw cameras in this view (off by default)");
            ui.checkbox(&mut d.lights, "Show Lights")
                .on_hover_text("Draw lights in this view (off by default)");
            ui.checkbox(&mut d.locators, "Show Locators")
                .on_hover_text("Draw Group Locators in this view");
            ui.checkbox(&mut d.symmetry_plane, "Show Symmetry Plane").on_hover_text(
                "With Symmetry on, outline the mirror plane across the active mesh. Off hides only the outline; Symmetry stays on",
            );
            ui.checkbox(&mut d.item_mode_wireframe, "Show Wireframe in Item Mode");
            ui.checkbox(&mut d.show_verts, "Show Verts")
                .on_hover_text("On: every point, in every mode. Off: Show Vertices decides.");
            ui.menu_button(
                format!("Show Vertices: {}", d.show_vertices.label()),
                |ui| {
                    for mode in ShowVertices::ALL {
                        ui.radio_value(&mut d.show_vertices, mode, mode.label());
                    }
                },
            );
            ui.checkbox(&mut d.uv_seams, "Show UV Seam")
                .on_hover_text("Edges where the first UV map's islands break");
            ui.checkbox(&mut d.checker, "Checker").on_hover_text(
                "Show a numbered grey and white check through the selected UV map: stretched UVs show as stretched squares",
            );
            ui.checkbox(&mut d.smoothing, "Smoothing")
                .on_hover_text("Off: every polygon faceted");
            // MT7-05 (missing-tools-g3): Topology Mode, view.topologyMode.
            ui.checkbox(&mut d.topology, "Topology Mode").on_hover_text(
                "Draw the active meshes over the background meshes, see-through, for retopology",
            );
            ui.separator();
            ui.label(egui::RichText::new("Advanced").small().weak());
            ui.checkbox(&mut d.shadows, "Show Shadows").on_hover_text(
                "Advanced style: shadows from the scene's lights (the view light when there are none)",
            );
            ui.separator();
            let modified = self.display.modified(&presets);
            for preset in &presets {
                let checked = self.display.applied_name() == Some(preset.name.as_str());
                let text = format!(
                    "{} {}{}",
                    if checked { "✔" } else { "   " },
                    preset.name,
                    if checked && modified { " *" } else { "" }
                );
                if ui
                    .add(Button::new(text).frame(false))
                    .on_hover_text("Choose again to restore the settings from before it")
                    .clicked()
                {
                    self.display.choose(preset);
                    self.status = match self.display.applied_name() {
                        Some(name) => format!("Viewport preset: {name}"),
                        None => "Viewport preset off; settings restored".into(),
                    };
                    ui.close();
                }
            }
            ui.separator();
            if ui.button("Save Viewport Preset…").clicked() {
                self.open_save_viewport_preset();
                ui.close();
            }
            let any = !self.view_presets.user.is_empty();
            if ui
                .gated(crate::unavailable::unless(any, "No saved presets; built-ins can't be deleted"), Button::new("Delete Preset…"))
                .clicked()
            {
                let names: Vec<String> =
                    self.view_presets.user.iter().map(|p| p.name.clone()).collect();
                let list: Vec<&str> = names.iter().map(String::as_str).collect();
                let rows = vec![
                    form::Row::choice("preset", "Preset", &list, 0)
                        .fresh()
                        .tip("The saved preset to delete; built-ins can't be deleted"),
                ];
                self.preset_dialog = Some(PresetDialog {
                    save: false,
                    form: self.form_dialog("Delete Preset", rows),
                    names,
                });
                ui.close();
            }
        });
        let rect = response.0.rect;
        response.0.on_hover_text(tip);
        if response.1.is_some() {
            self.menu_owned_input = true;
        }
        rect
    }

    pub(super) fn preset_dialog_window(&mut self, ctx: &Context) {
        let Some(mut dialog) = self.preset_dialog.take() else {
            return;
        };
        if dialog.save {
            let blank = dialog.form.values.text("name").trim().is_empty();
            dialog
                .form
                .block_ok(blank.then(|| "Type a name first".to_owned()));
        }
        match self.show_form(ctx, &mut dialog.form) {
            form::Outcome::Cancel => {
                self.status = "Viewport preset unchanged".into();
                return;
            }
            form::Outcome::Ok => {}
            _ => {
                self.preset_dialog = Some(dialog);
                return;
            }
        }
        let v = &dialog.form.values;
        let message = if dialog.save {
            let name = v.text("name").to_owned();
            let include = Include {
                drawing: v.bool("drawing"),
                active: v.bool("active"),
                inactive: v.bool("inactive"),
            };
            match self
                .view_presets
                .save(&name, &self.display.settings, include)
            {
                Ok(()) => {
                    self.remember_form(&dialog.form);
                    // The saved preset is the view's look now: check it.
                    self.display.applied = Some(view_display::Applied {
                        name: name.trim().to_string(),
                        before: self.display.settings,
                    });
                    format!("Saved viewport preset “{}”", name.trim())
                }
                Err(e) => {
                    self.status = e.clone();
                    dialog.form.error = Some(e);
                    self.preset_dialog = Some(dialog);
                    return;
                }
            }
        } else {
            let name = dialog
                .names
                .get(v.choice("preset"))
                .cloned()
                .unwrap_or_default();
            self.view_presets.delete(&name);
            // Every view that had it applied keeps its settings.
            for display in self.displays.values_mut().chain([&mut self.display]) {
                if display.applied_name() == Some(name.as_str()) {
                    display.applied = None;
                }
            }
            format!("Deleted viewport preset “{name}”")
        };
        self.status = match self
            .view_presets
            .save_to(&self.environment.paths().viewport_presets())
        {
            Ok(()) => message,
            Err(e) => format!("{message}; not saved to disk: {e}"),
        };
    }
}
