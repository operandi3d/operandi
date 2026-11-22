//! Distance units (Preferences > Input > Distance Units): how distances are
//! shown and which unit a typed number without one is taken in. Scene data
//! stays in metres; only text changes.
//!
//! The setting is per thread (the app sets it on its UI thread), so each
//! test thread sees the defaults unless it sets its own.
use serde::{Deserialize, Serialize};
use std::cell::Cell;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitSystem {
    /// Provisional default (UNI-Q1): metre-based, as Operandi always was.
    #[default]
    Si,
    Metric,
    English,
    GameUnits,
    Unitless,
}
impl UnitSystem {
    pub const ALL: [Self; 5] = [
        Self::Si,
        Self::Metric,
        Self::English,
        Self::GameUnits,
        Self::Unitless,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Si => "SI",
            Self::Metric => "Metric",
            Self::English => "English",
            Self::GameUnits => "Game Units",
            Self::Unitless => "Unitless",
        }
    }
    /// The Default Unit choices of this system.
    pub fn units(self) -> &'static [DistanceUnit] {
        use DistanceUnit::*;
        match self {
            Self::Si | Self::Metric => &[Micrometre, Millimetre, Centimetre, Metre, Kilometre],
            Self::English => &[Mil, Inch, Foot, Yard, Mile],
            Self::GameUnits => &[GameUnit],
            Self::Unitless => &[Plain],
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistanceUnit {
    Micrometre,
    Millimetre,
    Centimetre,
    #[default]
    Metre,
    Kilometre,
    Mil,
    Inch,
    Foot,
    Yard,
    Mile,
    GameUnit,
    /// Unitless: 1 = 1 m.
    Plain,
}
impl DistanceUnit {
    pub fn label(self) -> &'static str {
        match self {
            Self::Micrometre => "µm",
            Self::Millimetre => "mm",
            Self::Centimetre => "cm",
            Self::Metre => "m",
            Self::Kilometre => "km",
            Self::Mil => "mil",
            Self::Inch => "in",
            Self::Foot => "ft",
            Self::Yard => "yd",
            Self::Mile => "mi",
            Self::GameUnit => "gu",
            Self::Plain => "",
        }
    }
    /// Metres in one of this unit.
    pub fn metres(self, units: &DistanceUnits) -> f64 {
        match self {
            Self::Micrometre => 1e-6,
            Self::Millimetre => 0.001,
            Self::Centimetre => 0.01,
            Self::Metre | Self::Plain => 1.,
            Self::Kilometre => 1000.,
            Self::Mil => 0.0000254,
            Self::Inch => 0.0254,
            Self::Foot => 0.3048,
            Self::Yard => 0.9144,
            Self::Mile => 1609.344,
            Self::GameUnit => units.meters_per_game_unit,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DistanceUnits {
    pub system: UnitSystem,
    /// Provisional default (UNI-Q1): metres.
    pub default_unit: DistanceUnit,
    /// English: lengths up to this many inches show as inches. Measured: 36.
    pub inch_cutoff: u32,
    /// Provisional default (UNI-Q1): 1.
    pub meters_per_game_unit: f64,
}
impl Default for DistanceUnits {
    fn default() -> Self {
        Self {
            system: UnitSystem::Si,
            default_unit: DistanceUnit::Metre,
            inch_cutoff: 36,
            meters_per_game_unit: 1.,
        }
    }
}
impl DistanceUnits {
    pub fn valid(&self) -> bool {
        self.system.units().contains(&self.default_unit)
            && self.inch_cutoff <= 10_000
            && self.meters_per_game_unit.is_finite()
            && self.meters_per_game_unit > 0.
    }
    /// Keep the default unit one of the system's own.
    pub fn with_system(mut self, system: UnitSystem) -> Self {
        if self.system != system {
            self.system = system;
            self.default_unit = match system {
                UnitSystem::Si | UnitSystem::Metric => DistanceUnit::Metre,
                UnitSystem::English => DistanceUnit::Inch,
                UnitSystem::GameUnits => DistanceUnit::GameUnit,
                UnitSystem::Unitless => DistanceUnit::Plain,
            };
        }
        self
    }
    /// Distance text for `metres`, e.g. `1.5 m`, `250 mm`, `27"`, `3' 6"`.
    /// Provisional (UNI-Q1): SI, Metric and Game Units show the Default Unit.
    pub fn format(&self, metres: f64) -> String {
        match self.system {
            UnitSystem::English => {
                let inches = metres / 0.0254;
                if inches.abs() <= self.inch_cutoff as f64 + 1e-9 {
                    format!("{}\"", number(inches))
                } else {
                    let sign = if inches < 0. { "-" } else { "" };
                    let total = inches.abs();
                    let mut feet = (total / 12.).floor();
                    let mut rest = total - feet * 12.;
                    // 11.999995" rounds to a whole foot, not 12 inches.
                    if number(rest) == "12" {
                        feet += 1.;
                        rest = 0.;
                    }
                    format!("{sign}{}' {}\"", number(feet), number(rest))
                }
            }
            UnitSystem::Unitless => number(metres),
            _ => {
                let unit = self.default_unit;
                format!("{} {}", number(metres / unit.metres(self)), unit.label())
            }
        }
    }
}
/// Up to five decimals, trailing zeros trimmed (tiny values keep more).
fn number(value: f64) -> String {
    if value.abs() < 1e-12 {
        return "0".into();
    }
    let text = if value.abs() < 0.00001 {
        format!("{value:.12}")
    } else {
        format!("{value:.5}")
    };
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" {
        "0".into()
    } else {
        text.into()
    }
}

thread_local! {
    static CURRENT: Cell<DistanceUnits> = Cell::new(DistanceUnits::default());
}
/// The distance units this thread shows and reads.
pub fn current() -> DistanceUnits {
    CURRENT.with(Cell::get)
}
/// Set the distance units for this thread (the app's UI thread).
pub fn set_current(units: DistanceUnits) {
    if units.valid() {
        CURRENT.with(|c| c.set(units));
    }
}
/// Distance text in the current units.
pub fn format_distance(metres: f64) -> String {
    current().format(metres)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::numeric::parse;

    fn with<T>(units: DistanceUnits, f: impl FnOnce() -> T) -> T {
        let before = current();
        set_current(units);
        let out = f();
        set_current(before);
        out
    }

    #[test]
    fn defaults_show_and_read_metres() {
        assert_eq!(format_distance(1.5), "1.5 m");
        assert_eq!(format_distance(0.), "0 m");
        assert_eq!(parse("2").unwrap(), 2.);
        assert_eq!(parse("250 mm").unwrap(), 0.25);
    }

    #[test]
    fn default_unit_scales_numbers_typed_without_one() {
        let mm = DistanceUnits {
            default_unit: DistanceUnit::Millimetre,
            ..Default::default()
        };
        with(mm, || {
            assert_eq!(format_distance(0.25), "250 mm");
            assert!((parse("250").unwrap() - 0.25).abs() < 1e-12);
            // A unit anywhere means plain numbers are factors, not mm.
            assert_eq!(parse("1 m/4").unwrap(), 0.25);
            assert!((parse("2 cm").unwrap() - 0.02).abs() < 1e-12);
        });
    }

    #[test]
    fn english_shows_inches_up_to_the_cutoff_then_feet_and_inches() {
        let english = DistanceUnits::default().with_system(UnitSystem::English);
        assert_eq!(english.default_unit, DistanceUnit::Inch);
        with(english, || {
            assert_eq!(format_distance(27. * 0.0254), "27\"");
            assert_eq!(format_distance(36. * 0.0254), "36\"");
            assert_eq!(format_distance(42. * 0.0254), "3' 6\"");
            assert_eq!(format_distance(-48. * 0.0254), "-4' 0\"");
            // What it shows reads back.
            for inches in [5., 27., 42., 100.5] {
                let text = format_distance(inches * 0.0254);
                assert!(
                    (parse(&text).unwrap() / 0.0254 - inches).abs() < 1e-6,
                    "{text}"
                );
            }
            assert!((parse("2 ft").unwrap() - 0.6096).abs() < 1e-12);
            assert!((parse("1 yd").unwrap() - 0.9144).abs() < 1e-12);
            assert!(
                (parse("10").unwrap() - 0.254).abs() < 1e-12,
                "plain = inches"
            );
        });
        let cutoff = DistanceUnits {
            inch_cutoff: 12,
            ..english
        };
        assert_eq!(cutoff.format(27. * 0.0254), "2' 3\"");
    }

    #[test]
    fn game_units_and_unitless() {
        let game = DistanceUnits {
            meters_per_game_unit: 1.375,
            ..DistanceUnits::default().with_system(UnitSystem::GameUnits)
        };
        with(game, || {
            assert_eq!(format_distance(2.75), "2 gu");
            assert_eq!(parse("2").unwrap(), 2.75);
            assert_eq!(parse("1 gu").unwrap(), 1.375);
        });
        let plain = DistanceUnits::default().with_system(UnitSystem::Unitless);
        with(plain, || {
            assert_eq!(format_distance(2.5), "2.5");
            assert_eq!(parse("2.5").unwrap(), 2.5);
        });
    }

    #[test]
    fn invalid_settings_are_ignored() {
        let bad = DistanceUnits {
            default_unit: DistanceUnit::Foot,
            ..Default::default()
        };
        assert!(!bad.valid());
        with(bad, || assert_eq!(format_distance(1.), "1 m"));
    }
}
