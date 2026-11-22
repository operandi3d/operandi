//! The Absolute Scaling window (spec `snaps-precision-2026-11-10` A): a
//! small floating palette, opened from the Measure group's Absolute
//! Scaling button. Its fields stay for the session (A3) and every button
//! refreshes them afterwards (A6). Rows follow the property-panel rules
//! (UIC-110..116): one setting per row, so X, Y and Z are three rows
//! rather than the reference's one.
use super::*;
use operandi_core::{
    AbsoluteFields, AbsoluteMode, AbsoluteScale, ScaleCenter, UniformAxis,
    command_line::absolute_scale_arguments,
};

impl Operandi {
    /// The Absolute Scaling button: open the window, or close it (A2).
    pub(super) fn toggle_absolute_scaling(&mut self) {
        self.absolute_open = !self.absolute_open;
    }

    /// One button press: the scale, then the fields read back from the
    /// result. Grab Size only reads.
    pub(super) fn absolute_press(&mut self, mode: AbsoluteMode) -> bool {
        let scale = AbsoluteScale {
            mode,
            fields: self.absolute,
        };
        // The flat-axis note (Q2) is worked out before the points move.
        let note = self
            .editor
            .absolute_plan(scale)
            .ok()
            .and_then(|p| p.flat_note(scale));
        if mode != AbsoluteMode::Grab && !self.run_ok(Command::AbsoluteScale(scale)) {
            return false;
        }
        match self.editor.absolute_fields_after(self.absolute) {
            Ok(after) => {
                self.absolute = after;
                let s = after.size;
                self.status = note.unwrap_or_else(|| {
                    format!(
                        "{}: {} × {} × {}",
                        mode.label(),
                        create_utility::label_in_units(s.x),
                        create_utility::label_in_units(s.y),
                        create_utility::label_in_units(s.z)
                    )
                });
                true
            }
            Err(error) => {
                self.status = error;
                false
            }
        }
    }

    /// `scale.absolute mode …` from the command line or a script: given
    /// values become the window's, then it runs like the button.
    pub(super) fn command_absolute_scale(&mut self, args: &[String]) {
        match absolute_scale_arguments(args, self.absolute) {
            Ok((mode, fields)) => {
                self.absolute = fields;
                self.absolute_press(mode);
            }
            Err(error) => self.status = error,
        }
    }

    pub(super) fn absolute_window(&mut self, ctx: &egui::Context) {
        if !self.absolute_open {
            return;
        }
        let mut pressed = None;
        // Its own OS window like the Snaps palette (Jason, 2026-11-10);
        // about the reference's 250 × 350 (A2).
        let (open, _) = self.palette_window(
            ctx,
            "absolute",
            "Absolute",
            [270., 420.],
            None,
            |app, ui| {
                use crate::form::{Unit, rows};
                let f = &mut app.absolute;
                let d = AbsoluteFields::default();
                let big = 1e9;
                rows::scope(ui, "absolute-scaling", |ui| {
                    if rows::button(ui, "Grab Size")
                        .on_hover_text("Fill X, Y and Z (and Uniform) with the selection's size")
                        .clicked()
                    {
                        pressed = Some(AbsoluteMode::Grab);
                    }
                    rows::section(ui, "Uniform", false, |ui| {
                        if rows::button(ui, "Uniform Scale")
                            .on_hover_text(
                                "Scale every axis by one factor so the Uniform Axis becomes \
                                 Uniform long",
                            )
                            .clicked()
                        {
                            pressed = Some(AbsoluteMode::Uniform);
                        }
                        rows::number_d(
                            ui,
                            "Uniform",
                            &mut f.uniform,
                            d.uniform,
                            Unit::Distance,
                            -big..=big,
                            "The length the Uniform Axis is scaled to",
                        );
                        let axes: Vec<(UniformAxis, &str)> =
                            UniformAxis::ALL.iter().map(|a| (*a, a.label())).collect();
                        rows::choice_d(
                            ui,
                            "Uniform Axis",
                            &mut f.axis,
                            &d.axis,
                            &axes,
                            "Which side of the selection's box Uniform measures",
                        );
                        rows::check_d(
                            ui,
                            "Orient To Selection",
                            &mut f.orient,
                            d.orient,
                            "Measure the box along the selection's own sides, not the axes \
                             (provisional fit, Q4)",
                        );
                    });
                    rows::section(ui, "Explicit", false, |ui| {
                        if rows::button(ui, "Explicit Scale")
                            .on_hover_text(
                                "Scale each axis so the selection's box becomes X × Y × Z; \
                                 0 leaves that axis alone",
                            )
                            .clicked()
                        {
                            pressed = Some(AbsoluteMode::Explicit);
                        }
                        if rows::button(ui, "Reference Scale")
                            .on_hover_text(
                                "Measure the factors on the selection, then scale the whole \
                                 mesh by them",
                            )
                            .clicked()
                        {
                            pressed = Some(AbsoluteMode::Reference);
                        }
                        for (i, name) in ["X", "Y", "Z"].into_iter().enumerate() {
                            rows::number_d(
                                ui,
                                name,
                                &mut f.size[i],
                                0.,
                                Unit::Distance,
                                -big..=big,
                                "The size the selection's box gets on this axis \
                                 (negative mirrors, 0 leaves it alone)",
                            );
                        }
                    });
                    rows::section(ui, "Scaling Center", false, |ui| {
                        for (i, name) in
                            ["X Center", "Y Center", "Z Center"].into_iter().enumerate()
                        {
                            let axis = ["X", "Y", "Z"][i];
                            let labels = ScaleCenter::ALL.map(|c| format!("{} {axis}", c.label()));
                            let options: Vec<(ScaleCenter, &str)> = ScaleCenter::ALL
                                .iter()
                                .zip(&labels)
                                .map(|(c, l)| (*c, l.as_str()))
                                .collect();
                            rows::choice_d(
                                ui,
                                name,
                                &mut f.center[i],
                                &ScaleCenter::Center,
                                &options,
                                "What stays put on this axis: the box's middle, high or low side, or the \
                                 item's origin",
                            );
                        }
                    });
                });
            },
        );
        if !open {
            self.absolute_open = false;
        }
        if let Some(mode) = pressed {
            self.absolute_press(mode);
        }
    }
}
