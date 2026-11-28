//! SA-012: Spikey numeric configuration and discrete, cumulative Apply.
use super::*;
use crate::form::Unit;
use crate::tool_form::{self as tf, Button, ToolFields, ToolForm};

/// Spike's Tool Properties: one number, Spike Strength (SA-012).
pub(super) struct SpikeFields;

impl ToolFields for SpikeFields {
    type P = f64;
    fn params(app: &Operandi) -> Option<f64> {
        app.editor.session.spike.as_ref().map(|s| s.requested)
    }
    fn form(app: &Operandi, _: &f64) -> ToolForm<f64> {
        let mut f = ToolForm::new();
        f.title("Spikey")
            .field(
                tf::number(
                    "spi_strength",
                    "Spike Strength",
                    Unit::Scalar,
                    |p: &f64| *p,
                    |p, v| {
                        *p = v;
                        Ok(())
                    },
                )
                .suffix(""),
            )
            .wide(Button::command("Apply", Command::Apply).enabled_if(
                app.field_errors.is_empty(),
                crate::unavailable::FIELD_ERRORS,
            ))
            .section(
                "Tool States",
                true,
                // Spec: SA-Q04
                vec![tf::Item::Note(
                    "Tool State presets aren't available yet.".into(),
                )],
            )
            .buttons(vec![
                Button::command("Drop", Command::Drop),
                Button::command("Cancel", Command::Cancel),
                Button::command("Reset Attributes", Command::ResetSpike),
            ])
            .note(if app.editor.spike_step_newest() {
                // UIC-78: the last Apply stays live.
                "Applied · Strength reshapes these spikes; Apply or a click in the view spikes again; Q keeps them."
            } else {
                "Numeric Apply · each application spikes the current polygons; a click in the view applies too."
            });
        f
    }
    fn publish(app: &mut Operandi, p: f64) -> Result<()> {
        app.editor.dispatch(Command::ConfigureSpike(p)).map(|_| ())
    }
    fn published(app: &mut Operandi) {
        app.status = if app.editor.spike_step_newest() {
            "Spikes reshaped · Q keeps them".into()
        } else {
            "Spike Strength configured · Apply operates on the current polygons".into()
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> Operandi {
        let mut a = Operandi::empty();
        a.editor.dispatch(Command::NewMesh).unwrap();
        a.editor.scene.active_mut().unwrap().mesh = operandi_core::Mesh {
            vertices: vec![
                DVec3::new(-1., -1., 0.),
                DVec3::new(1., -1., 0.),
                DVec3::new(1., 1., 0.),
                DVec3::new(-1., 1., 0.),
            ],
            faces: vec![operandi_core::Face {
                vertices: vec![0, 1, 2, 3],
                uv: None,
                material: "Original".into(),
                smooth: false,
                part: None,
                material_label: None,
            }]
            .into(),
            ..Default::default()
        }
        .into();
        a.editor.history.undo.clear();
        a.execute("Spike");
        a
    }
    #[test]
    fn spi_sweep_inactive_fields_recovery_repeat_history_and_switching() {
        let mut a = app();
        let original = a.editor.scene.clone();
        a.fields.insert("spi_strength".into(), "1/2".into());
        assert!(a.confirm_fields());
        assert_eq!(a.editor.scene, original);
        a.run(Command::Apply);
        assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), 4);
        a.fields.insert("spi_strength".into(), "250 mm".into());
        assert!(!a.confirm_fields());
        let before = a.editor.scene.clone();
        a.run(Command::Apply);
        assert_eq!(a.editor.scene, before);
        // UIC-78: the first Apply is still live, so the new Strength
        // reshapes it in the same step.
        a.fields.insert("spi_strength".into(), "1/4".into());
        assert!(a.confirm_fields());
        let reshaped = a.editor.scene.clone();
        assert_ne!(reshaped, before);
        assert_eq!(a.editor.history.undo.len(), 1);
        a.execute("Spike Apply");
        assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), 12);
        a.execute("Undo");
        assert_eq!(a.editor.scene, reshaped);
        a.execute("Redo");
        assert!(a.editor.session.spike.is_none());
        assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), 12);
        a.execute("Spike");
        a.fields.insert("spi_strength".into(), "invalid".into());
        a.execute("Move");
        assert!(a.editor.session.spike.is_some());
        assert!(a.editor.session.live.is_none());
        a.fields.insert("spi_strength".into(), "0.25".into());
        a.execute("Move");
        assert!(a.editor.session.spike.is_none());
        assert!(a.editor.session.live.is_some());
    }
    #[test]
    fn spi_panel_menu_art_input_route_and_compact_field_reachability() {
        let mut a = app();
        let ctx = Context::default();
        theme::install(&ctx);
        for size in [vec2(1100., 720.), vec2(1440., 900.)] {
            let _ = ctx.run(
                RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.set_max_width(230.);
                        a.parameters(ui);
                    });
                },
            );
            assert!(a.field_rects.contains_key("spi_strength"));
            assert!(
                a.menu_model(&ctx)
                    .iter()
                    .flat_map(|m| m.leaves())
                    .any(|i| i.action == app_menu::line_action("Spike") && i.checked)
            );
        }
        assert!(a.bindings.get("Spike").is_some_and(String::is_empty));
        a.execute("spike strength -0.25");
        assert_eq!(a.editor.session.spike.as_ref().unwrap().requested, -0.25);
        a.execute("spike apply");
        assert_eq!(
            a.editor.scene.active().unwrap().mesh.vertices[4],
            DVec3::new(0., 0., -0.5)
        );
        a.fields.insert("spi_strength".into(), "invalid".into());
        assert!(!a.confirm_fields());
        a.run(Command::ResetSpike);
        assert!(a.field_errors.is_empty());
        assert_eq!(a.editor.session.spike.as_ref().unwrap().requested, 0.);
        // UIC-78: Cancel takes back the spike that is still live.
        a.run(Command::Cancel);
        assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), 1);
    }
}
