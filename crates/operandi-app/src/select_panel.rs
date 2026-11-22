//! Select tab of the toolbox (Jason, 2026-11-04): the selection commands in
//! one place, just below Create. Since layouts stage 3 batch 2 the tab is a
//! form, `assets/forms/toolbox.select.json` (toolbox_form.rs): its buttons
//! are command lines, greyed by their commands' own answers
//! (command_handlers.rs). Only the drop-downs a form can't draw yet live
//! here, as escape-hatch widgets (form/command_form.rs `WIDGETS`).
//!
//! The rows not built yet are `operandi.notBuilt` lines, greyed with "not in
//! Operandi yet". The Selection Sets rows run the bare select.editSet /
//! convertGroup / useSet / deleteSet lines, which open those commands'
//! dialogs (CMD-12); they grey for the reasons the dialogs refuse
//! (selection_set_dialog.rs `selection_set_unavailable`, UIC-223).
use super::*;
use crate::unavailable::Gate;

// The Select tab's escape-hatch widgets (form/command_form.rs `WIDGETS`):
// drop-downs a form can't draw yet.

/// Select Falloff: not built, a greyed drop-down.
pub(super) fn falloff_placeholder(ui: &mut Ui) {
    ui.gated_ui(Some("Select Falloff · not in Operandi yet"), |ui| {
        option_row::dropdown(ui, "select-falloff", "Select Falloff", "Linear", |_| ());
    });
}

impl Operandi {
    /// Polygon Tag Type (spec missing-tools-g1 MT1-01..03): what Materials
    /// selection mode picks by. The drop-down shows the live setting and a
    /// pick runs `operandi.select.polygonTagType` (UIC-164), the same line
    /// as the modes bar list and Select > Polygon Tag Type. It never
    /// changes the selection itself.
    pub(super) fn polygon_tag_type_field(&mut self, ui: &mut Ui) {
        let current = self.editor.polygon_tag_kind;
        let this = &*self;
        let chosen = option_row::dropdown(
            ui,
            "select-polygon-tag-type",
            "Polygon Tag Type",
            current.label(),
            |ui| this.tag_kind_list(ui),
        );
        chosen
            .response
            .on_hover_text("Polygon Tag Type · what Materials selection mode selects by");
        if let Some(kind) = chosen.inner {
            let name = match kind {
                operandi_core::PolygonTagKind::Material => "material",
                operandi_core::PolygonTagKind::Part => "part",
                operandi_core::PolygonTagKind::SmoothingGroup => "smoothingGroup",
                operandi_core::PolygonTagKind::SelectionSet => "selectionSet",
            };
            self.run_command_line(&format!("operandi.select.polygonTagType type:{name}"));
        }
    }

    /// Lasso Style: the shape a right-drag lasso draws.
    pub(super) fn lasso_style_field(&mut self, ui: &mut Ui) {
        let mut style = self.lasso_style;
        option_row::dropdown(
            ui,
            "select-lasso-style",
            "Lasso Style",
            style.label(),
            |ui| {
                for choice in lasso::Style::ALL {
                    ui.selectable_value(&mut style, choice, choice.label());
                }
            },
        )
        .response
        .on_hover_text("Lasso Style · the shape a right-drag lasso draws");
        self.lasso_style = style;
    }
}
