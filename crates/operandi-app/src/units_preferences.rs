//! Units and grid preferences (spec preferences-2026-11-04/UNITS-GRID.md):
//! Input > Distance Units, Input > Grid and Work Plane, and Viewport Fit
//! Factor from Display > OpenGL. Saved per user in `units-preferences.json`.
//! Values the spec has not measured yet are marked *provisional*.
use crate::prefs_file::PrefsFile;
use crate::unavailable::Gate;
use eframe::egui;
use operandi_core::units::{self, DistanceUnits, UnitSystem};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GridType {
    Fixed,
    /// Spacing follows zoom (provisional default: Operandi's grid so far).
    #[default]
    Dynamic,
}

pub const FIT_RANGE: std::ops::RangeInclusive<f64> = 0.5..=10.0;

/// Coordinate Rounding: how a pointer position on the work plane becomes
/// a coordinate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rounding {
    /// Raw values: Operandi so far, so the provisional default.
    #[default]
    None,
    /// Clean values, a step every ~3 points at the current zoom.
    Normal,
    /// About one step per point.
    Fine,
    /// Like Normal, but no finer than Fixed Increment (the grid too).
    Fixed,
    /// Always Fixed Increment, and the grid with it.
    ForcedFixed,
}
impl Rounding {
    pub const ALL: [Self; 5] = [
        Self::None,
        Self::Normal,
        Self::Fine,
        Self::Fixed,
        Self::ForcedFixed,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Normal => "Normal",
            Self::Fine => "Fine",
            Self::Fixed => "Fixed",
            Self::ForcedFixed => "Force Fixed",
        }
    }
}

/// Color Unit System: how colour components are shown and typed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorUnits {
    /// 0–1, may exceed 1. Provisional default: the fields so far.
    #[default]
    FloatingPoint,
    Percentage,
    /// 0–255.
    Integer,
    /// 00–ff per channel.
    Hexadecimal,
}
impl ColorUnits {
    pub const ALL: [Self; 4] = [
        Self::FloatingPoint,
        Self::Percentage,
        Self::Integer,
        Self::Hexadecimal,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::FloatingPoint => "Floating Point",
            Self::Percentage => "Percentage",
            Self::Integer => "Integer (0–255)",
            Self::Hexadecimal => "Hexadecimal",
        }
    }
}

/// Factor or Range: 0–1 factors as percentages or plain numbers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FactorDisplay {
    /// Provisional default: the fields so far.
    #[default]
    Percentage,
    Float,
}

/// The three work planes, by the index the views use (0 ground XZ, 1 YZ,
/// 2 XY).
pub const PLANES: [(u8, &str); 3] = [(0, "XZ (ground)"), (1, "YZ"), (2, "XY")];

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UnitsPreferences {
    version: u8,
    pub distance: DistanceUnits,
    pub grid_type: GridType,
    /// Fixed grid square size in metres. Provisional: 1 m.
    pub grid_size: f64,
    /// Fit All / Fit Selected margin. Measured: 1.5.
    pub fit_factor: f64,
    /// Preferred Plane the work plane leans toward. Provisional (GRD-Q1):
    /// XZ, the ground.
    pub preferred_plane: u8,
    /// Preferred Plane Bias, percent: 0 follows the view alone (expected
    /// default); 100 keeps the preferred plane almost always.
    pub preferred_bias: u8,
    /// Lock Preferred Plane: the work plane stays on it whatever the view.
    /// Provisional: off.
    pub lock_preferred: bool,
    /// Locked Plane Offset in metres along the locked plane's normal.
    /// Provisional: 0.
    pub locked_offset: f64,
    /// Plane and Grid Increments: how grid spacing steps with zoom.
    pub increments: crate::grid::Increments,
    pub rounding: Rounding,
    /// Fixed Increment in metres. Provisional: 10 mm (the spec's example).
    pub fixed_increment: f64,
    pub color_units: ColorUnits,
    /// Display colour values in display space (colour-nav A7, OP): colour
    /// numbers shown and typed display-encoded; storage stays linear. On by
    /// default, like the reference.
    pub color_display_space: bool,
    pub factor: FactorDisplay,
    /// Up Axis: 0 X, 1 Y, 2 Z. Expected default Y.
    pub up_axis: u8,
}
/// The units file's version. 2 (2026-11-21): Display Color Values in
/// Display Space became the default; a version 1 file only ever held the
/// old default, so loading it turns the option on once.
const UNITS_VERSION: u8 = 2;
impl Default for UnitsPreferences {
    fn default() -> Self {
        Self {
            version: UNITS_VERSION,
            distance: DistanceUnits::default(),
            grid_type: GridType::Dynamic,
            grid_size: 1.,
            fit_factor: 1.5,
            preferred_plane: 0,
            preferred_bias: 0,
            lock_preferred: false,
            locked_offset: 0.,
            increments: Default::default(),
            rounding: Rounding::None,
            fixed_increment: 0.01,
            color_units: ColorUnits::FloatingPoint,
            color_display_space: true,
            factor: FactorDisplay::Percentage,
            up_axis: 1,
        }
    }
}
impl UnitsPreferences {
    /// The rounding step for a point where one screen point covers `mpp`
    /// metres, or None for raw values.
    pub fn rounding_step(&self, mpp: f64) -> Option<f64> {
        let step = |points: f64| self.increments.step((mpp * points).max(1e-9));
        match self.rounding {
            Rounding::None => None,
            Rounding::Normal => Some(step(3.)),
            Rounding::Fine => Some(step(1.)),
            Rounding::Fixed => Some(step(3.).max(self.fixed_increment)),
            Rounding::ForcedFixed => Some(self.fixed_increment),
        }
    }
    /// The work plane the views use when the scene's isn't fixed, given
    /// the view direction `forward`: the axis plane most facing the view,
    /// leaning toward the Preferred Plane by its bias, or always it when
    /// locked.
    pub fn view_plane(&self, forward: glam::DVec3) -> u8 {
        if self.lock_preferred {
            return self.preferred_plane;
        }
        let mut a = forward.abs();
        let bias = f64::from(self.preferred_bias) / 100.;
        match self.preferred_plane {
            1 => a.x += bias,
            2 => a.z += bias,
            _ => a.y += bias,
        }
        if a.y >= a.x && a.y >= a.z {
            0
        } else if a.x >= a.z {
            1
        } else {
            2
        }
    }
    /// The fixed grid size, when the grid is fixed.
    pub fn fixed_grid(&self) -> Option<f64> {
        (self.grid_type == GridType::Fixed).then_some(self.grid_size)
    }
    /// Make these the units and grid this thread draws and reads with.
    pub fn apply(&self) {
        units::set_current(self.distance);
        crate::grid::set_fixed(self.fixed_grid());
        crate::grid::set_increments(self.increments);
        crate::grid::set_floor(match self.rounding {
            Rounding::Fixed => Some((self.fixed_increment, false)),
            Rounding::ForcedFixed => Some((self.fixed_increment, true)),
            _ => None,
        });
        crate::value_display::set(self.color_units, self.factor);
        crate::value_display::set_display_space(self.color_display_space);
        crate::camera::set_up_axis(self.up_axis);
    }
    /// Camera distance that frames a box with diagonal `diagonal` in a view
    /// `width` × `height`: at Fit Factor 1 the box's bounding sphere touches
    /// the view's shorter side.
    pub fn fit_distance(&self, diagonal: f64, width: f32, height: f32) -> f64 {
        // The projection maps one unit at distance 1 to 0.9 × view height, so
        // a sphere of radius R fills half the height at distance 1.8 R.
        let radius = diagonal.max(0.2) / 2.;
        let tall = (height / width.max(1.)).max(1.) as f64;
        radius * 1.8 * tall * self.fit_factor
    }
}
impl crate::prefs_file::PrefsFile for UnitsPreferences {
    const INVALID: &'static str = "Invalid units preference";
    fn valid(&self) -> bool {
        (1..=UNITS_VERSION).contains(&self.version)
            && self.distance.valid()
            && self.grid_size.is_finite()
            && (1e-6..=1e6).contains(&self.grid_size)
            && FIT_RANGE.contains(&self.fit_factor)
            && self.preferred_plane <= 2
            && self.preferred_bias <= 100
            && self.locked_offset.is_finite()
            && self.locked_offset.abs() <= 1e6
            && self.fixed_increment.is_finite()
            && (1e-6..=1e6).contains(&self.fixed_increment)
            && self.up_axis <= 2
    }
    fn upgrade(mut self) -> Self {
        if self.version < 2 {
            self.color_display_space = true;
        }
        self.version = UNITS_VERSION;
        self
    }
}

impl super::Operandi {
    /// The work plane point under `pos` (`Camera::plane_point`), rounded by
    /// Coordinate Rounding along the plane's two axes.
    pub(super) fn work_point(
        &self,
        pos: eframe::egui::Pos2,
        r: eframe::egui::Rect,
        plane: u8,
        center: glam::DVec3,
    ) -> Option<glam::DVec3> {
        let p = self.camera.plane_point(pos, r, plane, center)?;
        let Some(step) = self
            .camera
            .metres_per_pixel(p, r)
            .and_then(|mpp| self.units_preferences.rounding_step(mpp))
        else {
            return Some(p);
        };
        let round = |v: f64, c: f64| c + ((v - c) / step).round() * step;
        let mut q = p;
        match plane {
            1 => {
                q.y = round(p.y, center.y);
                q.z = round(p.z, center.z);
            }
            2 => {
                q.x = round(p.x, center.x);
                q.y = round(p.y, center.y);
            }
            _ => {
                q.x = round(p.x, center.x);
                q.z = round(p.z, center.z);
            }
        }
        Some(q)
    }
    /// Preferences > Input > Distance Units.
    pub(super) fn distance_units_page(&mut self, ui: &mut egui::Ui) {
        let before = self.units_preferences;
        let d = &mut self.units_preferences.distance;
        let dd = UnitsPreferences::default();
        use crate::form::{Unit, rows};
        rows::section(ui, "Distance Units", false, |ui| {
            let systems: Vec<_> = UnitSystem::ALL.iter().map(|&s| (s, s.label())).collect();
            let mut system = d.system;
            rows::choice_d(
                ui,
                "Unit System",
                &mut system,
                &dd.distance.system,
                &systems,
                "How distances are shown and typed",
            );
            *d = d.with_system(system);
            let units: Vec<_> = d.system.units().iter().map(|&u| (u, u.label())).collect();
            ui.gated_ui(
                crate::unavailable::unless(
                    d.system != UnitSystem::Unitless,
                    "Unitless numbers have no unit",
                ),
                |ui| {
                    rows::choice_d(
                        ui,
                        "Default Unit",
                        &mut d.default_unit,
                        &dd.distance.default_unit,
                        &units,
                        "The unit a number typed without one is in",
                    );
                },
            );
            ui.gated_ui(
                crate::unavailable::unless(
                    d.system == UnitSystem::English,
                    "Set Unit System to English first",
                ),
                |ui| {
                    rows::number_d(
                        ui,
                        "Inch Scale Cutoff",
                        &mut d.inch_cutoff,
                        dd.distance.inch_cutoff,
                        Unit::Suffix(" in"),
                        0..=10_000,
                        "Distances under this show in inches, above it in feet",
                    );
                },
            );
            ui.gated_ui(
                crate::unavailable::unless(
                    d.system == UnitSystem::GameUnits,
                    "Set Unit System to Game Units first",
                ),
                |ui| {
                    rows::number_d(
                        ui,
                        "Meters per Game Unit",
                        &mut d.meters_per_game_unit,
                        dd.distance.meters_per_game_unit,
                        Unit::Scalar,
                        0.0001..=10_000.,
                        "How many metres one game unit is",
                    );
                },
            );
        });
        let u = &mut self.units_preferences;
        rows::section(ui, "Accuracy and Units", false, |ui| {
            let roundings: Vec<_> = Rounding::ALL.iter().map(|&r| (r, r.label())).collect();
            rows::choice_d(
                ui,
                "Coordinate Rounding",
                &mut u.rounding,
                &dd.rounding,
                &roundings,
                "How a pointer position on the work plane becomes a coordinate",
            );
            let fixed = matches!(u.rounding, Rounding::Fixed | Rounding::ForcedFixed);
            ui.gated_ui(
                crate::unavailable::unless(
                    fixed,
                    "Set Coordinate Rounding to a Fixed option first",
                ),
                |ui| {
                    rows::number_d(
                        ui,
                        "Fixed Increment",
                        &mut u.fixed_increment,
                        dd.fixed_increment,
                        Unit::Distance,
                        1e-6..=1e6,
                        "The step coordinates round to with Fixed rounding",
                    );
                },
            );
            let colours: Vec<_> = ColorUnits::ALL.iter().map(|&c| (c, c.label())).collect();
            rows::choice_d(
                ui,
                "Color Unit System",
                &mut u.color_units,
                &dd.color_units,
                &colours,
                "How colour numbers are shown",
            );
            rows::check_d(
                ui,
                "Display Color Values in Display Space",
                &mut u.color_display_space,
                dd.color_display_space,
                "Color numbers show and take display-encoded (sRGB) values; \
                 colors are still stored linear",
            );
            rows::choice_d(
                ui,
                "Up Axis",
                &mut u.up_axis,
                &dd.up_axis,
                &[(0, "X"), (1, "Y"), (2, "Z")],
                "Which axis is up: orbiting, the ground grid, the Top view, and files \
                 read and written Y-up turn to it",
            );
            rows::choice_d(
                ui,
                "Factor or Range",
                &mut u.factor,
                &dd.factor,
                &[
                    (FactorDisplay::Percentage, "Percentage"),
                    (FactorDisplay::Float, "Float"),
                ],
                "Show 0–1 factors as percentages or plain numbers",
            );
        });
        crate::form::rows::note(
            ui,
            &format!(
                "Example: 1.25 m shows as {}. A number typed without a unit is in the \
                 Default Unit.",
                self.units_preferences.distance.format(1.25)
            ),
        );
        crate::form::rows::note(
            ui,
            "Not built yet: Input As Current Unit, and the dynamics and light units. \
             Up Axis doesn't turn primitives' default axes yet.",
        );
        self.save_units(before);
    }
    /// Preferences > Input > Grid and Work Plane (and Viewport Fit Factor).
    pub(super) fn grid_page(&mut self, ui: &mut egui::Ui) {
        let before = self.units_preferences;
        let u = &mut self.units_preferences;
        let d = UnitsPreferences::default();
        use crate::form::{Unit, rows};
        rows::section(ui, "Grid and Work Plane", false, |ui| {
            rows::choice_d(
                ui,
                "Grid Type",
                &mut u.grid_type,
                &d.grid_type,
                &[(GridType::Fixed, "Fixed"), (GridType::Dynamic, "Dynamic")],
                "Fixed keeps one spacing; Dynamic steps it as you zoom",
            );
            ui.gated_ui(
                crate::unavailable::unless(
                    u.grid_type == GridType::Fixed,
                    "Set Grid Type to Fixed first",
                ),
                |ui| {
                    rows::number_d(
                        ui,
                        "Size",
                        &mut u.grid_size,
                        d.grid_size,
                        Unit::Distance,
                        1e-6..=1e6,
                        "The fixed grid's spacing",
                    );
                },
            );
            rows::number_d(
                ui,
                "Viewport Fit Factor",
                &mut u.fit_factor,
                d.fit_factor,
                Unit::Scalar,
                FIT_RANGE,
                "How much room Fit leaves around what it frames",
            );
            rows::choice_d(
                ui,
                "Preferred Plane",
                &mut u.preferred_plane,
                &d.preferred_plane,
                &PLANES,
                "The plane the work plane leans toward",
            );
            rows::number_d(
                ui,
                "Preferred Plane Bias",
                &mut u.preferred_bias,
                d.preferred_bias,
                Unit::Percent,
                0..=100,
                "How strongly the work plane leans toward the preferred plane",
            );
            rows::check_d(
                ui,
                "Lock Preferred Plane",
                &mut u.lock_preferred,
                d.lock_preferred,
                "The work plane stays on the preferred plane",
            );
            ui.gated_ui(
                crate::unavailable::unless(u.lock_preferred, "Turn on Lock Preferred Plane first"),
                |ui| {
                    rows::number_d(
                        ui,
                        "Locked Plane Offset",
                        &mut u.locked_offset,
                        d.locked_offset,
                        Unit::Distance,
                        -1e6..=1e6,
                        "How far the locked plane sits from the origin",
                    );
                },
            );
            let increments: Vec<_> = crate::grid::Increments::ALL
                .iter()
                .map(|&i| (i, i.label()))
                .collect();
            rows::choice_d(
                ui,
                "Plane and Grid Increments",
                &mut u.increments,
                &d.increments,
                &increments,
                "How grid spacing steps as you zoom",
            );
        });
        crate::form::rows::note(
            ui,
            "A fixed grid switches to dynamic spacing when zoomed out so far that its \
             lines would crowd. Preferred Plane settings apply when the scene's work \
             plane isn't fixed. Grid Visibility and Flatness of Perspective are on \
             Display.",
        );
        self.save_units(before);
    }
    /// A Color Unit System chosen in the colour picker's Options is the
    /// same preference: save it.
    pub(super) fn take_picker_color_units(&mut self) {
        if let Some(units) = crate::value_display::take_picked_color_units() {
            let before = self.units_preferences;
            self.units_preferences.color_units = units;
            self.save_units(before);
        }
    }
    pub(super) fn save_units(&mut self, before: UnitsPreferences) {
        if self.units_preferences == before {
            return;
        }
        self.units_preferences.apply();
        // Interchange files follow the Up Axis.
        self.file_io_preferences.apply();
        // Shown distance text follows the new units.
        self.clear_fields();
        if let Err(error) = self
            .units_preferences
            .save(&self.environment.paths().units_preferences())
        {
            self.status = format!("Preference changed; could not save it: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::number_text;
    use std::fs;

    #[test]
    fn distance_text_follows_the_units() {
        assert_eq!(number_text(1.5, " m"), "1.5 m");
        let mm = UnitsPreferences {
            distance: DistanceUnits {
                default_unit: units::DistanceUnit::Millimetre,
                ..Default::default()
            },
            ..Default::default()
        };
        mm.apply();
        assert_eq!(number_text(0.25, " m"), "250 mm");
        // Other quantities are untouched.
        assert_eq!(number_text(25., "%"), "25%");
        UnitsPreferences::default().apply();
        assert_eq!(number_text(0.25, " m"), "0.25 m");
    }

    #[test]
    fn fit_factor_sets_the_framing_margin() {
        let p = UnitsPreferences::default();
        assert_eq!(p.fit_factor, 1.5);
        // Factor 1: the bounding sphere's radius fills half the height.
        let one = UnitsPreferences {
            fit_factor: 1.,
            ..p
        };
        let d = one.fit_distance(2., 1600., 900.);
        assert!((1. * 0.9 * 900. / d - 450.).abs() < 1e-6);
        assert!((p.fit_distance(2., 1600., 900.) / d - 1.5).abs() < 1e-12);
        // A tall view fits the width instead.
        assert!((one.fit_distance(2., 900., 1800.) / d - 2.).abs() < 1e-12);
    }

    #[test]
    fn fixed_grid_keeps_its_size_until_lines_would_crowd() {
        use crate::camera::Camera;
        use eframe::egui::{Rect, pos2};
        let r = Rect::from_min_max(pos2(0., 0.), pos2(1200., 900.));
        let mut camera = Camera::default();
        let dynamic = crate::grid::spacing(&camera, r, 0, glam::DVec3::ZERO);
        crate::grid::set_fixed(Some(0.25));
        assert_eq!(crate::grid::spacing(&camera, r, 0, glam::DVec3::ZERO), 0.25);
        camera.distance *= 1000.;
        assert!(crate::grid::spacing(&camera, r, 0, glam::DVec3::ZERO) > 0.25);
        crate::grid::set_fixed(None);
        camera.distance /= 1000.;
        assert_eq!(
            crate::grid::spacing(&camera, r, 0, glam::DVec3::ZERO),
            dynamic
        );
    }

    #[test]
    fn preferences_roundtrip_and_reject_bad_values() {
        let dir = std::env::temp_dir().join(format!("operandi-units-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("units-preferences.json");
        assert_eq!(UnitsPreferences::load(&path), UnitsPreferences::default());
        let p = UnitsPreferences {
            distance: DistanceUnits::default().with_system(UnitSystem::English),
            grid_type: GridType::Fixed,
            grid_size: 0.5,
            ..Default::default()
        };
        p.save(&path).unwrap();
        assert_eq!(UnitsPreferences::load(&path), p);
        assert!(
            UnitsPreferences {
                fit_factor: 0.,
                ..p
            }
            .save(&path)
            .is_err()
        );
        fs::remove_dir_all(dir).unwrap();
    }

    /// Display space is on by default. A version 1 file (written when off
    /// was the default) turns it on once; a later choice to turn it off
    /// sticks.
    #[test]
    fn display_space_defaults_on_and_old_files_turn_it_on_once() {
        assert!(UnitsPreferences::default().color_display_space);
        let dir = std::env::temp_dir().join(format!("operandi-units-v1-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("units-preferences.json");
        let mut old = serde_json::to_value(UnitsPreferences::default()).unwrap();
        old["version"] = 1.into();
        old["color_display_space"] = false.into();
        old["grid_size"] = 0.5.into();
        fs::write(&path, old.to_string()).unwrap();
        let loaded = UnitsPreferences::load(&path);
        assert!(loaded.color_display_space);
        assert_eq!(loaded.grid_size, 0.5);
        let off = UnitsPreferences {
            color_display_space: false,
            ..loaded
        };
        off.save(&path).unwrap();
        assert!(!UnitsPreferences::load(&path).color_display_space);
        fs::remove_dir_all(dir).unwrap();
    }

    // App route: a typed field value without a unit uses the Default Unit.
    #[test]
    fn typed_distances_use_the_default_unit() {
        let mm = DistanceUnits {
            default_unit: units::DistanceUnit::Millimetre,
            ..Default::default()
        };
        units::set_current(mm);
        assert!((operandi_core::numeric::parse("500").unwrap() - 0.5).abs() < 1e-12);
        units::set_current(DistanceUnits::default());
    }
}
