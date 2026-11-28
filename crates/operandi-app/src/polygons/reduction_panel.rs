//! Polygon Reduce panel (poly-reduce-2026-11-05): Number, Boundary Weight and
//! Material Border Weight, Apply, Drop and explicit Cancel. There is no
//! Legacy Mode (decided Q-RD-03). The fields and `tool.attr poly.reduct`
//! share one Configure route; geometry, source and history stay in Editor.
use super::*;
use crate::form::Unit;
use crate::tool_form::{self as tf, Button, ToolFields, ToolForm};

pub(super) const NUMBER: &str = "red_number";
const BOUNDARY: &str = "red_boundary";
const MATERIAL: &str = "red_material";
const NO_LEGACY: &str = "Operandi's Polygon Reduce has no Legacy Mode";

impl Operandi {
    /// One Configure route for the field and the property command.
    pub(super) fn configure_reduction_number(&mut self, number: u32) -> Result<u32> {
        self.configure_reduction_with(|p| p.number = number)?;
        Ok(number)
    }
    /// Boundary (`material` false) or Material Border Weight, in percent.
    fn configure_reduction_weight(&mut self, material: bool, percent: f64) -> Result<f64> {
        self.configure_reduction_with(|p| {
            if material {
                p.material_border_weight = percent;
            } else {
                p.boundary_weight = percent;
            }
        })?;
        Ok(percent)
    }
    fn configure_reduction_with(
        &mut self,
        change: impl FnOnce(&mut ReductionParameters),
    ) -> Result<()> {
        let mut p = self
            .editor
            .session
            .reduction
            .as_ref()
            .ok_or("Activate Polygon Reduction first")?
            .parameters;
        change(&mut p);
        self.editor.dispatch(Command::ConfigureReduction(p))?;
        self.status = self.reduction_status();
        Ok(())
    }
    /// Target, current and actual counts are reported separately (RED-002).
    pub(super) fn reduction_status(&self) -> String {
        let Some(s) = &self.editor.session.reduction else {
            return "Ready".into();
        };
        let requested = s.parameters.number;
        match (s.current_count(), s.achieved) {
            (None, _) if requested == 0 => {
                "Polygon Reduction armed · Number 0 · click the mesh to initialize".into()
            }
            (None, _) => format!(
                "Polygon Reduction armed · target {requested} · Apply initializes from the selected mesh"
            ),
            (Some(current), Some(actual)) if actual != requested => format!(
                "Polygon Reduction · current {current} · requested {requested} · actual {actual} (no further valid reduction)"
            ),
            (Some(current), Some(actual)) => {
                format!("Polygon Reduction · current {current} · result {actual} triangles")
            }
            (Some(current), None) => format!("Polygon Reduction · current {current}"),
        }
    }
    /// `tool.attr poly.reduct <attribute> <value|?>`.
    pub(super) fn reduction_property(&mut self, attr: &str, value: &str) {
        let Some(s) = self.editor.session.reduction.as_ref() else {
            self.status = "Activate Polygon Reduction first".into();
            return;
        };
        match attr {
            "number" if value == "?" => self.status = s.parameters.number.to_string(),
            "number" => {
                let parsed = numeric::parse_quantity(value, numeric::Quantity::Scalar)
                    .and_then(ReductionParameters::number_from_value);
                match parsed.and_then(|n| self.configure_reduction_number(n)) {
                    Ok(n) => {
                        self.sync_numeric_value(NUMBER, n as f64, "");
                        self.field_errors.remove(NUMBER);
                    }
                    Err(e) => self.status = format!("Number: {e}"),
                }
            }
            "boundaryweight" if value == "?" => {
                self.status = format!("{} %", s.parameters.boundary_weight)
            }
            "materialweight" if value == "?" => {
                self.status = format!("{} %", s.parameters.material_border_weight)
            }
            "boundaryweight" | "materialweight" => {
                let material = attr == "materialweight";
                // Script units: 0–1, or a value with % (map percent).
                let parsed = operandi_core::command_names::script_percent(value);
                match parsed.and_then(|v| self.configure_reduction_weight(material, v)) {
                    Ok(v) => {
                        let key = if material { MATERIAL } else { BOUNDARY };
                        self.sync_numeric_value(key, v, "");
                        self.field_errors.remove(key);
                    }
                    Err(e) => self.status = format!("{attr}: {e}"),
                }
            }
            "method" => self.status = NO_LEGACY.into(),
            _ => self.status = "Unknown owner or attribute; use help tool.attr".into(),
        }
    }
}

/// Polygon Reduction's Tool Properties (poly-reduce-2026-11-05).
pub(super) struct ReductionFields;

impl ToolFields for ReductionFields {
    type P = ReductionParameters;
    fn params(app: &Operandi) -> Option<ReductionParameters> {
        app.editor.session.reduction.as_ref().map(|s| s.parameters)
    }
    fn form(app: &Operandi, _: &ReductionParameters) -> ToolForm<ReductionParameters> {
        let Some(s) = &app.editor.session.reduction else {
            return ToolForm::new();
        };
        let mut f = ToolForm::new();
        f.title("Reduct")
            .field(
                tf::number(
                    NUMBER,
                    "Number",
                    Unit::Scalar,
                    |p: &ReductionParameters| p.number as f64,
                    |p, v| {
                        p.number = ReductionParameters::number_from_value(v)?;
                        Ok(())
                    },
                )
                .suffix(""),
            )
            .field(tf::number(
                BOUNDARY,
                "Boundary Weight",
                Unit::Percent,
                |p: &ReductionParameters| p.boundary_weight,
                |p, v| {
                    p.boundary_weight = v;
                    Ok(())
                },
            ))
            .field(tf::number(
                MATERIAL,
                "Material Border Weight",
                Unit::Percent,
                |p: &ReductionParameters| p.material_border_weight,
                |p, v| {
                    p.material_border_weight = v;
                    Ok(())
                },
            ))
            .wide(
                Button::run("Apply", |app| {
                    if app.run_ok(Command::Apply) {
                        app.status = app.reduction_status();
                    }
                })
                .enabled_if(app.field_errors.is_empty(), crate::unavailable::FIELD_ERRORS),
            )
            // Spec: PT-RED-05 / PT-Q13 (named tool states unmeasured)
            .unbuilt(
                "Tool States",
                "(none)",
                "Named Reduction tool states aren't available yet",
            )
            .buttons(vec![
                Button::command("Drop", Command::Drop),
                Button::command("Cancel", Command::Cancel)
                    .tip("Discard only the current uncommitted preview"),
            ])
            .note(match (s.current_count(), s.achieved) {
                (None, _) => "Armed · Number 0 until a stationary click in the viewport initializes the selected mesh".to_string(),
                (Some(current), Some(actual)) => {
                    format!("Current {current} · Result {actual} triangles")
                }
                (Some(current), None) => format!("Current {current}"),
            });
        f
    }
    fn publish(app: &mut Operandi, p: ReductionParameters) -> Result<()> {
        app.editor.dispatch(Command::ConfigureReduction(p))?;
        app.status = app.reduction_status();
        Ok(())
    }
    fn refusal_status(error: &str) -> String {
        format!("Number: {error}")
    }
}
