//! How colour components and 0–1 factors are shown and typed
//! (Preferences > Input > Distance Units: Color Unit System, Factor or
//! Range). The values themselves stay 0–1; only the fields change.
use crate::units_preferences::{ColorUnits, FactorDisplay};
use eframe::egui::{Response, Ui};
use std::cell::Cell;

thread_local! {
    static CURRENT: Cell<(ColorUnits, FactorDisplay)> =
        const { Cell::new((ColorUnits::FloatingPoint, FactorDisplay::Percentage)) };
    /// "Display colour values in display space" (A7): colour numbers are
    /// shown and typed display-encoded; stored values stay linear.
    static DISPLAY_SPACE: Cell<bool> = const { Cell::new(true) };
    /// A Color Unit System chosen in the colour picker's Options, for the
    /// app to write into Preferences (it is the same preference).
    static PICKED_UNITS: Cell<Option<ColorUnits>> = const { Cell::new(None) };
}
pub fn set(color: ColorUnits, factor: FactorDisplay) {
    CURRENT.with(|c| c.set((color, factor)));
}
fn current() -> (ColorUnits, FactorDisplay) {
    CURRENT.with(Cell::get)
}
/// Factors show as percentages (else as plain 0–1 numbers).
pub fn factor_is_percent() -> bool {
    current().1 == FactorDisplay::Percentage
}
pub fn color_units() -> ColorUnits {
    current().0
}
/// The colour picker's Options changed the Color Unit System: show it at
/// once and let the app save it.
pub fn pick_color_units(units: ColorUnits) {
    CURRENT.with(|c| c.set((units, c.get().1)));
    PICKED_UNITS.with(|p| p.set(Some(units)));
}
/// A unit choice made in the picker since the last call.
pub fn take_picked_color_units() -> Option<ColorUnits> {
    PICKED_UNITS.with(Cell::take)
}
pub fn set_display_space(on: bool) {
    DISPLAY_SPACE.with(|c| c.set(on));
}
pub fn display_space() -> bool {
    DISPLAY_SPACE.with(Cell::get)
}

/// A stored (linear) component as the number the fields show.
pub fn to_shown(value: f64) -> f64 {
    if display_space() {
        operandi_core::colour_math::linear_to_srgb(value.max(0.))
    } else {
        value
    }
}
/// A shown number back to the stored (linear) component.
pub fn from_shown(value: f64) -> f64 {
    if display_space() {
        operandi_core::colour_math::srgb_to_linear(value.max(0.))
    } else {
        value
    }
}

/// One colour component as text in `units` (the swatch numbers, A1/A6).
/// Hexadecimal clips to ff; the others show HDR values above the top.
pub fn format_color(value: f64, units: ColorUnits) -> String {
    format_color_raw(to_shown(value), units)
}

/// `format_color` without the display-space conversion (values that are
/// already display-encoded, such as theme colours).
pub fn format_color_raw(v: f64, units: ColorUnits) -> String {
    match units {
        ColorUnits::FloatingPoint => format!("{v:.2}"),
        ColorUnits::Percentage => format!("{:.1}%", v * 100.),
        ColorUnits::Integer => format!("{}", (v * 255.).round().max(0.) as u64),
        ColorUnits::Hexadecimal => format!("{:02x}", (v.clamp(0., 1.) * 255.).round() as u64),
    }
}

/// A colour component field over `value` (0–1, up to `max`).
pub fn color_component(ui: &mut Ui, value: &mut f64, max: f64) -> Response {
    match current().0 {
        ColorUnits::FloatingPoint => ui.add(
            crate::numeric_tab::DragValue::new(value)
                .range(0.0..=max)
                .speed(0.005)
                .max_decimals(2),
        ),
        ColorUnits::Percentage => scaled(ui, value, max, 100., "%", 1),
        ColorUnits::Integer => scaled(ui, value, max, 255., "", 0),
        ColorUnits::Hexadecimal => {
            let mut shown = (*value * 255.).round();
            let response = ui.add(
                crate::numeric_tab::DragValue::new(&mut shown)
                    .range(0.0..=(max * 255.).round())
                    .speed(1.)
                    .custom_formatter(|v, _| format!("{:02x}", v.round().max(0.) as u64))
                    .custom_parser(parse_hex),
            );
            if response.changed() {
                *value = shown / 255.;
            }
            response
        }
    }
}

/// A 0–1 factor field (`lo`..=`hi` in factor terms).
pub fn factor(ui: &mut Ui, value: &mut f64, lo: f64, hi: f64) -> Response {
    match current().1 {
        FactorDisplay::Percentage => {
            let mut shown = *value * 100.;
            let response = ui.add(
                crate::numeric_tab::DragValue::new(&mut shown)
                    .range(lo * 100.0..=hi * 100.0)
                    .speed(0.5)
                    .max_decimals(1)
                    .suffix("%"),
            );
            if response.changed() {
                *value = (shown / 100.).clamp(lo, hi);
            }
            response
        }
        FactorDisplay::Float => ui.add(
            crate::numeric_tab::DragValue::new(value)
                .range(lo..=hi)
                .speed(0.005)
                .max_decimals(2),
        ),
    }
}

/// Typed hexadecimal (`80`, `#ff`) as 0–255.
fn parse_hex(text: &str) -> Option<f64> {
    u64::from_str_radix(text.trim().trim_start_matches('#'), 16)
        .ok()
        .map(|v| v as f64)
}

fn scaled(
    ui: &mut Ui,
    value: &mut f64,
    max: f64,
    scale: f64,
    suffix: &str,
    decimals: usize,
) -> Response {
    let mut shown = *value * scale;
    let response = ui.add(
        crate::numeric_tab::DragValue::new(&mut shown)
            .range(0.0..=max * scale)
            .speed(if decimals == 0 { 1. } else { 0.5 })
            .max_decimals(decimals)
            .suffix(suffix),
    );
    if response.changed() {
        *value = shown / scale;
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{self, Context};

    #[test]
    fn every_unit_choice_draws_without_changing_the_value() {
        let ctx = Context::default();
        for colour in ColorUnits::ALL {
            for f in [FactorDisplay::Percentage, FactorDisplay::Float] {
                set(colour, f);
                let (mut c, mut k) = (0.5, 0.25);
                let _ = ctx.run(Default::default(), |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        color_component(ui, &mut c, 1.);
                        factor(ui, &mut k, 0., 1.);
                    });
                });
                assert_eq!((c, k), (0.5, 0.25), "{colour:?} {f:?}");
            }
        }
        set(ColorUnits::FloatingPoint, FactorDisplay::Percentage);
        assert_eq!(parse_hex("ff"), Some(255.));
        assert_eq!(parse_hex(" #80"), Some(128.));
        assert_eq!(parse_hex("zz"), None);
    }

    /// CN-06: each Color Unit System shows the same value.
    #[test]
    fn cn06_units_format_and_parse() {
        // Stored values shown as they are (display space off).
        set_display_space(false);
        let shown: Vec<String> = ColorUnits::ALL
            .iter()
            .map(|&u| format_color(0.5, u))
            .collect();
        assert_eq!(shown, ["0.50", "50.0%", "128", "80"]);
        // HDR shows above the top except in hex, which clips.
        assert_eq!(format_color(2., ColorUnits::Integer), "510");
        assert_eq!(format_color(2., ColorUnits::Hexadecimal), "ff");
        // Display space: numbers are encoded, storage stays linear.
        set_display_space(true);
        assert_eq!(format_color(0.214_041, ColorUnits::FloatingPoint), "0.50");
        assert!((from_shown(0.5) - 0.214_041).abs() < 1e-5);
    }
}
