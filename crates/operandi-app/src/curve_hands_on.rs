//! The Curve tab's hands-on commands in the app (CURVE-COMMANDS-2D-SHAPES,
//! hands-on 2026-11-17; core in `operandi_core::curve_convert`):
//!
//! - Convert Curves (CC-50/51): the curves become a new Converted_<type>
//!   item and the target curve tool opens on them in Edit mode.
//! - Curve offsets (CC-40): "Enter Offset Amount" with one Offset field.
//! - Bezier Tangent (CC-60): Operation, Tangent, Set Length and Length.
use super::*;
use operandi_core::curve_convert::{CurveType, TangentOperation, TangentOptions, TangentSide};

/// Which offset button opened the dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct OffsetKind {
    pub closed: bool,
    pub trio: bool,
}

pub(super) enum CurveDialog {
    Offset(OffsetKind, form::Dialog),
    Tangent(form::Dialog),
    Boolean(form::Dialog),
}

fn named<'a>(rest: &'a str, key: &str) -> Option<&'a str> {
    rest.split_whitespace().find_map(|w| {
        let (k, v) = w.split_once(':')?;
        k.eq_ignore_ascii_case(key).then_some(v.trim_matches('"'))
    })
}

fn truthy(v: Option<&str>, default: bool) -> bool {
    v.map_or(default, |v| {
        matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "on")
    })
}

/// The draw tool that edits curves of `t`.
fn draw_kind(t: CurveType) -> draw_tools::DrawKind {
    match t {
        CurveType::Curve => draw_tools::DrawKind::Curve,
        CurveType::Bezier => draw_tools::DrawKind::Bezier,
        CurveType::BSpline => draw_tools::DrawKind::BSpline,
    }
}

/// CC-54: why `curve.convert from:<type>` can't run now, if it can't.
pub(super) fn convert_state(a: &Operandi, rest: &str) -> Option<String> {
    let from = CurveType::named(named(rest, "from").unwrap_or("")).ok()?;
    let s = &a.editor.scene.selection;
    let Some(item) = a.editor.scene.active() else {
        return Some("Select a mesh with curves".into());
    };
    let faces = if s.mode == Mode::Polygon {
        s.faces.clone()
    } else {
        Vec::new()
    };
    operandi_core::curve_convert::convert_unavailable(&item.mesh, &faces, from)
}

impl Operandi {
    /// `curve.convert from:<type> to:<type>` (CC-50): convert, then open
    /// the target curve tool on the new curves in Edit mode.
    pub(super) fn convert_curves_line(&mut self, rest: &str) {
        let parse = |key: &str| CurveType::named(named(rest, key).unwrap_or(""));
        let (from, to) = match (parse("from"), parse("to")) {
            (Ok(f), Ok(t)) => (f, t),
            (Err(e), _) | (_, Err(e)) => {
                self.status = format!("curve.convert: {e}; use from:curve|bezier|bspline to:…");
                return;
            }
        };
        if self.run_ok(Command::ConvertCurves { from, to }) {
            self.activate_draw(draw_kind(to));
            self.status = format!(
                "{} to {}: a new item; the {} tool edits it (Q to finish)",
                from.label(),
                to.label(),
                to.label()
            );
        }
    }

    /// `ppi.curveOffset`: with an `offset:` it runs; without, it opens
    /// Enter Offset Amount for the closed/open, single/Trio button.
    pub(super) fn curve_offset_line(&mut self, rest: &str) {
        let kind = OffsetKind {
            closed: truthy(named(rest, "closed"), true),
            trio: truthy(named(rest, "trio"), false),
        };
        if let Some(v) = named(rest, "offset") {
            match form::Unit::Distance.parse(v) {
                Ok(offset) => {
                    self.run(Command::OffsetCurves {
                        closed: kind.closed,
                        offset,
                        trio: kind.trio,
                    });
                }
                Err(e) => self.status = format!("ppi.curveOffset: {e}"),
            }
            return;
        }
        if self.curve_dialog.is_some() || self.has_modal() {
            self.status = "Close the open dialog first".into();
            return;
        }
        let rows = vec![
            form::Row::number("offset", "Offset", form::Unit::Distance, 0.).tip(
                "How far each point moves along its corner: positive goes inside a closed curve",
            ),
        ];
        let dialog = form::Dialog::new(
            "Enter Offset Amount",
            rows.clone(),
            form::Values::defaults(&rows),
        );
        self.curve_dialog = Some(CurveDialog::Offset(kind, dialog));
    }

    /// `bezier.tangent`: bare opens the dialog; with words it runs.
    pub(super) fn bezier_tangent_line(&mut self, rest: &str) {
        if rest.trim().is_empty() {
            self.open_bezier_tangent();
            return;
        }
        let word = |k: &str, d: &str| named(rest, k).unwrap_or(d).to_ascii_lowercase();
        let operation = match word("operation", "sharp").as_str() {
            "sharp" | "0" => TangentOperation::Sharp,
            "smooth" | "1" => TangentOperation::Smooth,
            "uniform" | "2" => TangentOperation::Uniform,
            other => {
                self.status = format!("bezier.tangent: no operation {other}");
                return;
            }
        };
        let tangent = match word("tangent", "both").as_str() {
            "both" | "0" => TangentSide::Both,
            "out" | "1" => TangentSide::Out,
            "in" | "2" => TangentSide::In,
            other => {
                self.status = format!("bezier.tangent: no tangent {other}");
                return;
            }
        };
        let length = match named(rest, "length").map(|v| form::Unit::Distance.parse(v)) {
            Some(Ok(v)) => v,
            Some(Err(e)) => {
                self.status = format!("bezier.tangent: {e}");
                return;
            }
            None => 1.,
        };
        self.run(Command::BezierTangent(TangentOptions {
            operation,
            tangent,
            set_length: truthy(named(rest, "setLength"), false),
            length,
        }));
    }

    /// `curve.boolean` (CB-xx): bare opens its dialog; with words it runs.
    pub(super) fn curve_boolean_line(&mut self, rest: &str) {
        use operandi_core::curve_boolean::{CurveBooleanOp, CurveBooleanOptions};
        if rest.trim().is_empty() {
            if self.curve_dialog.is_some() || self.has_modal() {
                self.status = "Close the open dialog first".into();
                return;
            }
            use form::Row;
            let rows = vec![
                Row::choice("operation", "Operation", &CurveBooleanOp::NAMES, 0)
                    .tip("Slice adds points where the curves cross; the others combine the areas"),
                Row::choice("axis", "Axis", &["Auto", "X", "Y", "Z"], 0)
                    .tip("The plane the curves are cut in: Auto fits it to the curves"),
                Row::check("clip", "Clip", false)
                    .tip("Keep only the selected curves' own pieces, left open"),
                Row::check("merge", "Merge Curves", true)
                    .tip("Join the pieces into closed outlines; off leaves them separate"),
            ];
            let dialog = self.form_dialog("Curve Boolean", rows);
            self.curve_dialog = Some(CurveDialog::Boolean(dialog));
            return;
        }
        let op = match CurveBooleanOp::named(named(rest, "operation").unwrap_or("slice")) {
            Ok(op) => op,
            Err(e) => {
                self.status = e;
                return;
            }
        };
        let axis = match named(rest, "axis")
            .unwrap_or("auto")
            .to_ascii_lowercase()
            .as_str()
        {
            "x" | "1" => 1,
            "y" | "2" => 2,
            "z" | "3" => 3,
            _ => 0,
        };
        self.run(Command::CurveBoolean(CurveBooleanOptions {
            op,
            axis,
            clip: truthy(named(rest, "clip"), false),
            merge: truthy(named(rest, "merge"), true),
        }));
    }

    /// `bezier.tangent` bare: its dialog (CC-60).
    pub(super) fn open_bezier_tangent(&mut self) {
        if self.curve_dialog.is_some() || self.has_modal() {
            self.status = "Close the open dialog first".into();
            return;
        }
        use form::{Row, Unit, When};
        let rows = vec![
            Row::choice("operation", "Operation", &["Sharp", "Smooth", "Uniform"], 0)
                .tip("Sharp: a corner; Smooth: handles on one line; Uniform: on one line, the same length"),
            Row::choice("tangent", "Tangent", &["Both", "Out", "In"], 0)
                .tip("Which handles of the selected knots change"),
            Row::check("set_length", "Set Length", false).tip("Also give the handles Length"),
            Row::number("length", "Length", Unit::Distance, 1.)
                .when(When::Checked("set_length"))
                .tip("The handles' length with Set Length on"),
        ];
        let dialog = self.form_dialog("Bezier Tangent", rows);
        self.curve_dialog = Some(CurveDialog::Tangent(dialog));
    }

    /// The open dialog, once per frame.
    pub(super) fn curve_dialog_frame(&mut self, ctx: &Context) {
        let Some(dialog) = self.curve_dialog.take() else {
            return;
        };
        match dialog {
            CurveDialog::Offset(kind, mut form) => match self.show_form(ctx, &mut form) {
                form::Outcome::Cancel => self.status = "Curve Offset canceled".into(),
                form::Outcome::Ok => {
                    let offset = form.values.number("offset");
                    if !self.run_ok(Command::OffsetCurves {
                        closed: kind.closed,
                        offset,
                        trio: kind.trio,
                    }) {
                        form.error = Some(self.status.clone());
                        self.curve_dialog = Some(CurveDialog::Offset(kind, form));
                    }
                }
                _ => self.curve_dialog = Some(CurveDialog::Offset(kind, form)),
            },
            CurveDialog::Boolean(mut form) => match self.show_form(ctx, &mut form) {
                form::Outcome::Cancel => self.status = "Curve Boolean canceled".into(),
                form::Outcome::Ok => {
                    use operandi_core::curve_boolean::{CurveBooleanOp, CurveBooleanOptions};
                    let v = &form.values;
                    let o = CurveBooleanOptions {
                        op: CurveBooleanOp::from_index(v.choice("operation")),
                        axis: v.choice("axis") as u8,
                        clip: v.bool("clip"),
                        merge: v.bool("merge"),
                    };
                    let rows = form.rows.clone();
                    self.dialog_memory
                        .remember("Curve Boolean", &rows, &form.values);
                    if !self.run_ok(Command::CurveBoolean(o)) {
                        form.error = Some(self.status.clone());
                        self.curve_dialog = Some(CurveDialog::Boolean(form));
                    }
                }
                _ => self.curve_dialog = Some(CurveDialog::Boolean(form)),
            },
            CurveDialog::Tangent(mut form) => match self.show_form(ctx, &mut form) {
                form::Outcome::Cancel => self.status = "Bezier Tangent canceled".into(),
                form::Outcome::Ok => {
                    let v = &form.values;
                    let o = TangentOptions {
                        operation: match v.choice("operation") {
                            1 => TangentOperation::Smooth,
                            2 => TangentOperation::Uniform,
                            _ => TangentOperation::Sharp,
                        },
                        tangent: match v.choice("tangent") {
                            1 => TangentSide::Out,
                            2 => TangentSide::In,
                            _ => TangentSide::Both,
                        },
                        set_length: v.bool("set_length"),
                        length: v.number("length"),
                    };
                    let rows = form.rows.clone();
                    self.dialog_memory
                        .remember("Bezier Tangent", &rows, &form.values);
                    if !self.run_ok(Command::BezierTangent(o)) {
                        form.error = Some(self.status.clone());
                        self.curve_dialog = Some(CurveDialog::Tangent(form));
                    }
                }
                _ => self.curve_dialog = Some(CurveDialog::Tangent(form)),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offset_and_tangent_lines_open_their_dialogs_and_convert_opens_the_tool() {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.execute("ppi.curveOffset closed:true trio:true");
        assert!(a.curve_dialog.is_some(), "{}", a.status);
        assert!(matches!(
            a.curve_dialog,
            Some(CurveDialog::Offset(
                OffsetKind {
                    closed: true,
                    trio: true
                },
                _
            ))
        ));
        a.curve_dialog = None;
        a.execute("bezier.tangent");
        assert!(matches!(a.curve_dialog, Some(CurveDialog::Tangent(_))));
        a.curve_dialog = None;
        // A closed Curve, converted to Bezier: a new item, edited by the
        // Bezier tool.
        a.editor.scene.active_mut().unwrap().mesh = operandi_core::curve_ops::shape_mesh(
            operandi_core::curve_ops::Shape::Circle,
            operandi_core::curve_ops::ShapeType::Spline,
            glam::DVec2::ZERO,
            glam::DVec2::splat(0.5),
            1,
        )
        .into();
        a.run(Command::Mode(Mode::Polygon));
        a.editor.scene.selection.faces = vec![0];
        assert!(convert_state(&a, "from:curve to:bezier").is_none());
        assert!(convert_state(&a, "from:bezier to:curve").is_some(), "CC-54");
        let items = a.editor.scene.items.len();
        a.execute("curve.convert from:curve to:bezier");
        assert_eq!(a.editor.scene.items.len(), items + 1, "{}", a.status);
        assert_eq!(a.editor.scene.active().unwrap().name, "Converted_Bezier");
        assert_eq!(
            a.draw.as_ref().map(|d| d.kind),
            Some(draw_tools::DrawKind::Bezier)
        );
    }
}
