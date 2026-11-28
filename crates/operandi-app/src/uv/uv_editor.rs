//! The UV Editor window and the toolbox's UV tab (UVT-200..203,
//! uv-editing-2026-11-16 section 0). The UV layout itself is a layout file
//! (assets/layouts/uv.json, layouts.rs).
//!
//! - UVT-200 / 203 UV Editor: Texture > Open UV Editor, or double-click a
//!   map in the UV Maps list, opens a UV view in its own OS window (UIC-183:
//!   floating windows are OS windows) about 800 × 600, with the common UV
//!   tools down its right side, whatever layout the main window shows.
//! - UVT-201: the toolbox's UV tab (the Map field at its top is the same
//!   choice as the UV view's map button), opened by the UV layout's open
//!   command `operandi.toolbox.tab uv`. The tab is a form now
//!   (`assets/forms/toolbox.uv.json`, toolbox_form.rs); the Map field is
//!   its custom widget `uv.mapField`.
use super::*;
use crate::unavailable::Gate;

/// Width of the UV Editor's tool strip.
const STRIP: f32 = 150.;

impl Operandi {
    /// A double-click in the UV Maps list (UVT-200, LAY-54): the UV Editor
    /// palette opens, or the open one comes forward.
    pub(super) fn open_uv_editor(&mut self) {
        if let Err(e) = self.set_palette_open(crate::palettes::UV_EDITOR, true) {
            self.status = e;
        }
    }

    /// Texture > Open UV Editor (UVT-200/203, LAY-54, UIC-207): the UV
    /// Editor palette, a UV view in its own OS window (UIC-183), toggles
    /// like any palette (LAY-50).
    pub(super) fn toggle_uv_editor(&mut self) {
        self.toggle_palette(crate::palettes::UV_EDITOR);
    }

    /// Whether `view` is the UV Editor window's view.
    pub(super) fn is_uv_editor(&self, view: u32) -> bool {
        self.palettes.views.get(crate::palettes::UV_EDITOR) == Some(&view)
    }

    /// The UV Editor's tool strip down its right side (UVT-200): the
    /// transform tools, then Unwrap, Pack and the island buttons.
    pub(super) fn uv_editor_strip(&mut self, ui: &mut Ui, rect: Rect) {
        let strip = Rect::from_min_max(pos2(rect.right() - STRIP, rect.top() + 34.), rect.max);
        ui.painter().rect_filled(strip, 0., theme::panel());
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(strip.shrink(6.)));
        let ui = &mut child;
        ui.spacing_mut().item_spacing.y = 3.;
        for tool in [Tool::Move, Tool::Rotate, Tool::Scale, Tool::Transform] {
            self.toolbox_button(
                ui,
                tool,
                vec2(ui.available_width(), toolbox::row_height()),
                false,
            );
        }
        ui.separator();
        self.uv_panel(ui);
    }

    /// Width the UV Editor's strip takes from its view.
    pub(super) fn uv_editor_strip_width(&self, view: u32) -> f32 {
        if self.is_uv_editor(view) { STRIP } else { 0. }
    }

    /// The UV tab's Map field (UVT-201): the same choice as the UV view's
    /// map button.
    pub(super) fn uv_map_field(&mut self, ui: &mut Ui) {
        let (maps, current) = match self.editor.scene.active() {
            Some(i) => (
                i.mesh.uv_maps.keys().cloned().collect::<Vec<_>>(),
                operandi_core::uv_maps::current_map(&i.mesh),
            ),
            None => (Vec::new(), None),
        };
        let idle = self.editor.session.live.is_none();
        let mut pick = None;
        ui.horizontal(|ui| {
            ui.label("Map");
            egui::ComboBox::from_id_salt("uv-tab-map")
                .width(ui.available_width() - 4.)
                .selected_text(current.as_deref().unwrap_or("(none)"))
                .show_ui(ui, |ui| {
                    for m in &maps {
                        if ui
                            .gated(
                                crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE),
                                Button::selectable(current.as_deref() == Some(m.as_str()), m),
                            )
                            .clicked()
                        {
                            pick = Some(operandi_core::uv_maps::UvMapEdit::Select(m.clone()));
                        }
                    }
                    if !maps.is_empty() {
                        ui.separator();
                    }
                    if ui
                        .gated(
                            crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE),
                            Button::new("New Map"),
                        )
                        .clicked()
                    {
                        pick = Some(operandi_core::uv_maps::UvMapEdit::New(None));
                    }
                });
        });
        if let Some(edit) = pick {
            self.run(Command::UvMap(edit));
        }
    }
}
