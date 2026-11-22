//! The Curve tab's commands in the app (CURVE-COMMANDS-2D-SHAPES v1): the
//! toolbox form is `assets/forms/toolbox.curve.json`; Make Open / Closed
//! Curve, Make Curve Fill and Align Winding run in the core
//! (`operandi_core::curve_commands`). Set Curves Color (CC-30) is here: a
//! display setting of the view, never part of the scene, so it records no
//! undo step and the saved file doesn't change.
use super::*;

/// The open Set Curves Color dialog.
pub(super) struct CurvesColorDialog {
    pub(super) form: form::Dialog,
}

/// The colour a curve takes when Override is on and no colour was given.
pub(super) const DEFAULT_CURVE_COLOUR: [u8; 3] = [255, 140, 40];

fn rows(on: bool, rgb: [u8; 3]) -> Vec<form::Row> {
    use form::Row;
    let colour = rgb.map(|c| c as f64 / 255.);
    vec![
        Row::check("override", "Override", on).tip(
            "Draw every curve in this view in the colour below; off uses the usual wire colour",
        ),
        Row::colour("color", "Color", colour).tip("The colour curves draw in while Override is on"),
    ]
    .into_iter()
    .map(form::Row::fresh)
    .collect()
}

/// `"r g b"` (0–1 each, or 0–255) as a colour.
fn parse_colour(text: &str) -> Option<[u8; 3]> {
    let parts: Vec<f64> = text
        .split([' ', ','])
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<f64>().ok())
        .collect::<Option<_>>()?;
    let [r, g, b] = parts.as_slice() else {
        return None;
    };
    let scale = if [r, g, b].iter().all(|v| **v <= 1.) {
        255.
    } else {
        1.
    };
    let c = |v: f64| (v * scale).round().clamp(0., 255.) as u8;
    Some([c(*r), c(*g), c(*b)])
}

impl Operandi {
    /// `curves.setColor`: bare opens the dialog; `override:` and `color:`
    /// set the view's curve colour directly.
    pub(super) fn curves_set_color_line(&mut self, rest: &str) {
        if rest.trim().is_empty() {
            self.open_curves_color();
            return;
        }
        let words = operandi_core::command_line::tokens(rest).unwrap_or_default();
        let mut on = true;
        let mut colour = self.curve_color.unwrap_or(DEFAULT_CURVE_COLOUR);
        for w in &words {
            let (key, value) = w.split_once(':').unwrap_or(("override", w));
            match key.to_ascii_lowercase().as_str() {
                "override" => {
                    on = matches!(value.to_ascii_lowercase().as_str(), "true" | "1" | "on")
                }
                "color" | "colour" => match parse_colour(value) {
                    Some(c) => colour = c,
                    None => {
                        self.status = "curves.setColor: give color as \"r g b\"".into();
                        return;
                    }
                },
                _ => {
                    self.status = format!("curves.setColor doesn't take {key}");
                    return;
                }
            }
        }
        self.set_curves_color(on.then_some(colour));
    }

    fn set_curves_color(&mut self, colour: Option<[u8; 3]>) {
        self.curve_color = colour;
        self.status = match colour {
            Some(_) => "Curves draw in the override colour in this view".into(),
            None => "Curves draw in the usual wire colour".into(),
        };
    }

    pub(super) fn open_curves_color(&mut self) {
        if self.curves_color.is_some() {
            return;
        }
        if self.has_modal() {
            self.status = "Close the open dialog before Set Curves Color".into();
            return;
        }
        let rgb = self.curve_color.unwrap_or(DEFAULT_CURVE_COLOUR);
        let rows = rows(self.curve_color.is_some(), rgb);
        // The view's own values, not the last OK's (it is a view setting).
        let values = form::Values::defaults(&rows);
        self.curves_color = Some(CurvesColorDialog {
            form: form::Dialog::new("Set Curves Color", rows, values),
        });
    }

    /// The dialog, once per frame.
    pub(super) fn curves_color_dialog(&mut self, ctx: &Context) {
        let Some(mut dialog) = self.curves_color.take() else {
            return;
        };
        match self.show_form(ctx, &mut dialog.form) {
            form::Outcome::Cancel => self.status = "Set Curves Color canceled".into(),
            form::Outcome::Ok => {
                let v = &dialog.form.values;
                let rgb = v
                    .colour("color")
                    .map(|c| (c * 255.).round().clamp(0., 255.) as u8);
                self.set_curves_color(v.bool("override").then_some(rgb));
            }
            _ => self.curves_color = Some(dialog),
        }
    }

    /// The colour a curve element draws in here, when the view overrides it.
    pub(super) fn curve_override(&self) -> Option<Color32> {
        self.curve_color.map(|[r, g, b]| Color32::from_rgb(r, g, b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sc1_the_colour_is_a_view_setting_with_no_undo_step() {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        let before = a.editor.scene.clone();
        let steps = a.editor.undo.len();
        a.execute("curves.setColor override:true color:\"0 1 0\"");
        assert_eq!(a.curve_color, Some([0, 255, 0]));
        assert_eq!(a.curve_override(), Some(Color32::from_rgb(0, 255, 0)));
        assert_eq!(a.editor.scene, before, "the scene doesn't change");
        assert_eq!(a.editor.undo.len(), steps, "no undo step");
        a.execute("curves.setColor override:false");
        assert_eq!(a.curve_color, None);
        a.execute("curves.setColor");
        assert!(a.curves_color.is_some(), "bare opens the dialog");
    }

    /// The Curve tab is in the toolbox strip, and clicking it shows its
    /// form (Jason's report, 2026-11-17: the tab was missing).
    #[test]
    fn the_curve_tab_is_in_the_strip_and_opens_its_form() {
        let mut a = Operandi::empty();
        let ctx = Context::default();
        theme::install(&ctx);
        let size = egui::vec2(1440., 900.);
        super::super::tests::draw(&mut a, &ctx, size, vec![]);
        let tab = a.toolbox_rects["Category Curve"].center();
        let click = |down: bool| egui::Event::PointerButton {
            pos: tab,
            button: egui::PointerButton::Primary,
            pressed: down,
            modifiers: egui::Modifiers::NONE,
        };
        super::super::tests::draw(
            &mut a,
            &ctx,
            size,
            vec![egui::Event::PointerMoved(tab), click(true), click(false)],
        );
        super::super::tests::draw(&mut a, &ctx, size, vec![]);
        assert_eq!(a.category, toolbox::Category::Curve);
        assert!(
            a.form_rects.contains_key("poly.makeCurveClosed"),
            "{:?}",
            a.form_rects.keys()
        );
    }

    #[test]
    fn colours_read_as_fractions_or_bytes() {
        assert_eq!(parse_colour("1 0 0.5"), Some([255, 0, 128]));
        assert_eq!(parse_colour("10,20,30"), Some([10, 20, 30]));
        assert_eq!(parse_colour("1 2"), None);
    }
}
