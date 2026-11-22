//! The Light Path Expressions output's rows in the Render Output panel
//! (render outputs group D, 2026-11-20): the expression field, its presets
//! beside it, and the status line under it, updated as you type (never a
//! modal error). Greyed for the other output types with the reason
//! (UIC-117), like Cryptomatte's rows.
use crate::Operandi;
use crate::unavailable::{Gate, unless};
use eframe::egui::{self, ComboBox, RichText, Ui};
use operandi_core::light_path_expressions::{self as lpe, Status};
use operandi_core::render_outputs::OutputSettings;
use operandi_core::shader_tree::OutputEffect;

impl Operandi {
    /// The expression, presets and status rows; edits `s` in place (the
    /// panel turns a change into one undo step).
    pub(crate) fn lpe_rows(
        &self,
        ui: &mut Ui,
        effect: Option<OutputEffect>,
        s: &mut OutputSettings,
    ) {
        let is_lpe = effect == Some(OutputEffect::LightPathExpressions);
        let presets = self.lpe_presets();
        ui.gated_ui(
            unless(is_lpe, "Set the Effect to Light Path Expressions first"),
            |ui| {
                crate::option_row::field_row(ui, "Light Path Expression", |ui| {
                    let response = ui
                        .add(
                            egui::TextEdit::singleline(&mut s.lpe)
                                .hint_text("Type an expression")
                                .desired_width(120.),
                        )
                        .on_hover_text(
                            "Which light reaches the camera: C.* all light, C<RD>L direct \
                             diffuse, C.*L'key' the lights labelled key",
                        );
                    crate::numeric_tab::register(ui, &response, Some(s.lpe.as_str()));
                    // UIC-55: a small arrow at the field's right opens its
                    // presets; picking one fills the field. Fixed presets
                    // only: no (new) / (delete) yet (reported to Jason).
                    ComboBox::from_id_salt("output-lpe-presets")
                        .selected_text("")
                        .width(18.)
                        .show_ui(ui, |ui| {
                            ui.set_min_width(220.);
                            for (name, text) in &presets {
                                if ui
                                    .selectable_label(s.lpe == *text, name.as_str())
                                    .on_hover_text(text.as_str())
                                    .clicked()
                                {
                                    s.lpe = text.clone();
                                }
                            }
                        })
                        .response
                        .on_hover_text("Light Path Expression presets");
                });
                if is_lpe {
                    let recipe = lpe::recipe(&s.lpe);
                    let line = recipe.status.line();
                    let text = match recipe.status {
                        Status::Exact => RichText::new(line).small(),
                        _ => RichText::new(line).small().weak(),
                    };
                    let response = ui.label(text);
                    if let Some(near) = recipe.fallback {
                        response.on_hover_text(format!(
                            "Renders black. The nearest exact expression: {near}"
                        ));
                    }
                }
            },
        );
    }

    /// The fixed presets, then "All Lights" and one entry per LPE Label of
    /// the scene's lights and environment.
    fn lpe_presets(&self) -> Vec<(String, String)> {
        let scene = &self.editor.scene;
        let mut labels = operandi_cycles::scene_light_groups(scene);
        for env in scene.items.iter().filter_map(|i| i.environment.as_ref()) {
            let l = env.lpe_label.trim();
            if !l.is_empty() && !labels.iter().any(|x| x == l) {
                labels.push(l.to_owned());
            }
        }
        lpe::PRESETS
            .iter()
            .map(|(n, t)| ((*n).to_owned(), (*t).to_owned()))
            .chain(lpe::label_presets(&labels))
            .collect()
    }
}
