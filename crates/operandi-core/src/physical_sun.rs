//! Physical Sun (LP-SUN-01..12, Tier 2): a directional light aimed where the
//! real sun is for a place, date and time, using NOAA's public "General
//! Solar Position" formulas (fractional-year series for the equation of
//! time and declination, then hour angle, zenith and azimuth). Accurate to
//! a few tenths of a degree, ignoring refraction.
//!
//! Scene orientation: north is −Z turned by North Offset about +Y, east is
//! +X at offset 0, up is +Y. The light travels from the sun toward the
//! scene (its −Z), so its −Z is the negated sun direction.
//!
//! Provisional (LPROP-Q12..Q14): the light is left out of renders while the
//! sun is below the horizon; Clamp Intensity scales Radiant Exitance by a
//! clear-sky air-mass curve (Clamp) or uses an absolute irradiance (None);
//! Haze thickens that curve's air.
use crate::Result;
use glam::{DQuat, DVec3};
use serde::{Deserialize, Serialize};

/// LP-SUN-06.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SunClamp {
    /// Physically real (very bright) sunlight.
    None,
    /// Real sunlight's rise and fall, capped at the light's Radiant
    /// Exitance.
    #[default]
    Clamp,
    /// The light's Radiant Exitance, whatever the sun's height.
    Replace,
}

impl SunClamp {
    pub const ALL: [SunClamp; 3] = [SunClamp::None, SunClamp::Clamp, SunClamp::Replace];
    pub fn label(self) -> &'static str {
        match self {
            SunClamp::None => "None",
            SunClamp::Clamp => "Clamp",
            SunClamp::Replace => "Replace",
        }
    }
}

/// A directional light's Physical Sun section.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PhysicalSun {
    pub enabled: bool,
    /// Local clock time, hours (12.5 = 12:30).
    pub time: f64,
    /// Day of the year as month (1..=12) and day of the month.
    pub month: u32,
    pub day: u32,
    pub daylight_saving: bool,
    /// 1..=10.
    pub haze: f64,
    pub clamp: SunClamp,
    /// LP-SUN-07, stored only (Tier 3).
    pub gamma: f64,
    /// Radians, from −Z.
    pub north_offset: f64,
    /// Degrees, −90..=90 (south negative).
    pub latitude: f64,
    /// Degrees, −180..=180 (west negative).
    pub longitude: f64,
    /// Hours from UTC.
    pub time_zone: f64,
}

impl Default for PhysicalSun {
    fn default() -> Self {
        Self {
            enabled: false,
            time: 12.0,
            month: 6,
            day: 21,
            daylight_saving: false,
            haze: 3.0,
            clamp: SunClamp::Clamp,
            gamma: 2.2,
            north_offset: 0.0,
            latitude: 37.53,
            longitude: -122.30,
            time_zone: -8.0,
        }
    }
}

/// LP-SUN-09: Operandi's own short list of places (LPQ-04): name, north
/// latitude, east longitude (degrees), standard time zone (hours).
pub const LOCATIONS: [(&str, f64, f64, f64); 12] = [
    ("San Francisco", 37.53, -122.30, -8.0),
    ("Los Angeles", 34.05, -118.24, -8.0),
    ("New York", 40.71, -74.01, -5.0),
    ("Rio de Janeiro", -22.91, -43.17, -3.0),
    ("London", 51.51, -0.13, 0.0),
    ("Paris", 48.86, 2.35, 1.0),
    ("Berlin", 52.52, 13.40, 1.0),
    ("Cape Town", -33.92, 18.42, 2.0),
    ("Mumbai", 19.08, 72.88, 5.5),
    ("Singapore", 1.35, 103.82, 8.0),
    ("Tokyo", 35.68, 139.69, 9.0),
    ("Sydney", -33.87, 151.21, 10.0),
];

const DAYS_IN_MONTH: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// Where the sun is: azimuth clockwise from north and elevation above the
/// horizon, degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SunPosition {
    pub azimuth: f64,
    pub elevation: f64,
}

impl PhysicalSun {
    /// The day of the year, 1-based (a non-leap year).
    pub fn day_of_year(&self) -> u32 {
        let m = self.month.clamp(1, 12) as usize;
        DAYS_IN_MONTH[..m - 1].iter().sum::<u32>() + self.day.clamp(1, DAYS_IN_MONTH[m - 1])
    }

    /// The named place whose numbers these are, if any.
    pub fn location(&self) -> Option<&'static str> {
        LOCATIONS
            .iter()
            .find(|(_, lat, lon, tz)| {
                (lat - self.latitude).abs() < 1e-9
                    && (lon - self.longitude).abs() < 1e-9
                    && (tz - self.time_zone).abs() < 1e-9
            })
            .map(|l| l.0)
    }

    /// NOAA General Solar Position.
    pub fn position(&self) -> SunPosition {
        use std::f64::consts::PI;
        // Clock time to standard time.
        let hours = self.time - if self.daylight_saving { 1.0 } else { 0.0 };
        let g = 2.0 * PI / 365.0 * (self.day_of_year() as f64 - 1.0 + (hours - 12.0) / 24.0);
        let eqtime = 229.18
            * (0.000075 + 0.001868 * g.cos()
                - 0.032077 * g.sin()
                - 0.014615 * (2.0 * g).cos()
                - 0.040849 * (2.0 * g).sin());
        let decl = 0.006918 - 0.399912 * g.cos() + 0.070257 * g.sin() - 0.006758 * (2.0 * g).cos()
            + 0.000907 * (2.0 * g).sin()
            - 0.002697 * (3.0 * g).cos()
            + 0.00148 * (3.0 * g).sin();
        let offset = eqtime + 4.0 * self.longitude - 60.0 * self.time_zone;
        let true_solar = hours * 60.0 + offset;
        let hour_angle = (true_solar / 4.0 - 180.0).to_radians();
        let lat = self.latitude.to_radians();
        let cos_zenith =
            (lat.sin() * decl.sin() + lat.cos() * decl.cos() * hour_angle.cos()).clamp(-1.0, 1.0);
        let zenith = cos_zenith.acos();
        let elevation = 90.0 - zenith.to_degrees();
        // Azimuth from the components of the sun direction (east, north),
        // which avoids the formula's poles at the zenith.
        let east = -decl.cos() * hour_angle.sin();
        let north = lat.cos() * decl.sin() - lat.sin() * decl.cos() * hour_angle.cos();
        let azimuth = east.atan2(north).to_degrees().rem_euclid(360.0);
        SunPosition { azimuth, elevation }
    }

    /// The unit vector toward the sun in scene space.
    pub fn sun_direction(&self) -> DVec3 {
        let p = self.position();
        let (a, e) = (p.azimuth.to_radians(), p.elevation.to_radians());
        let local = DVec3::new(a.sin() * e.cos(), e.sin(), -a.cos() * e.cos());
        DQuat::from_rotation_y(self.north_offset) * local
    }

    /// Cycles sun strength for a light whose Radiant Exitance is
    /// `exitance` (LP-SUN-06): None = absolute clear-sky irradiance; Clamp
    /// = that curve relative to the sun overhead, times `exitance`;
    /// Replace = `exitance`. 0 below the horizon.
    pub fn strength(&self, exitance: f64) -> f64 {
        let e = self.position().elevation;
        if e <= 0.0 {
            return 0.0;
        }
        let irradiance = |elevation: f64| {
            // Kasten–Young air mass, Meinel clear-sky transmittance;
            // Haze 3 is the clear sky, higher thickens the air.
            let z = 90.0 - elevation;
            let am = 1.0 / (z.to_radians().cos() + 0.50572 * (96.07995 - z).powf(-1.6364));
            let tau = self.haze.clamp(1.0, 10.0) / 3.0;
            1353.0 * 0.7f64.powf(tau * am.powf(0.678)) * elevation.to_radians().sin()
        };
        match self.clamp {
            SunClamp::None => irradiance(e),
            SunClamp::Clamp => exitance * irradiance(e) / irradiance(90.0),
            SunClamp::Replace => exitance,
        }
    }

    pub fn validate(&self) -> Result<()> {
        let ok = self.time.is_finite()
            && (0.0..24.0).contains(&self.time)
            && (1..=12).contains(&self.month)
            && (1..=DAYS_IN_MONTH[self.month.clamp(1, 12) as usize - 1]).contains(&self.day)
            && (1.0..=10.0).contains(&self.haze)
            && self.gamma.is_finite()
            && (0.1..=10.0).contains(&self.gamma)
            && self.north_offset.is_finite()
            && (-90.0..=90.0).contains(&self.latitude)
            && (-180.0..=180.0).contains(&self.longitude)
            && (-14.0..=14.0).contains(&self.time_zone);
        if ok {
            Ok(())
        } else {
            Err("Invalid Physical Sun settings".into())
        }
    }
}

/// The world rotation Physical Sun gives a directional light, when on.
pub fn driven_sun_rotation(light: &crate::camera_light::LightSettings) -> Option<DQuat> {
    if light.kind != crate::camera_light::LightKind::Directional || !light.sun.enabled {
        return None;
    }
    crate::item_target::aim_rotation(-light.sun.sun_direction(), 0.0)
}

/// One Physical Sun row.
#[derive(Clone, Debug, PartialEq)]
pub enum SunEdit {
    Enabled(bool),
    Time(f64),
    /// Month and day.
    Day(u32, u32),
    DaylightSaving(bool),
    Haze(f64),
    Clamp(SunClamp),
    NorthOffset(f64),
    /// Index into `LOCATIONS`.
    Location(usize),
    /// A stored place (LP-SUN-13): latitude, longitude, time zone.
    Place(f64, f64, f64),
    /// LP-SUN-07.
    Gamma(f64),
    Latitude(f64),
    Longitude(f64),
    TimeZone(f64),
}

impl SunEdit {
    pub fn label(&self) -> &'static str {
        match self {
            SunEdit::Enabled(_) => "Physical Sun",
            SunEdit::Time(_) => "Time",
            SunEdit::Day(..) => "Day",
            SunEdit::DaylightSaving(_) => "Daylight Savings Time",
            SunEdit::Haze(_) => "Haze",
            SunEdit::Clamp(_) => "Clamp Intensity",
            SunEdit::NorthOffset(_) => "North Offset",
            SunEdit::Location(_) | SunEdit::Place(..) => "Location",
            SunEdit::Gamma(_) => "Gamma",
            SunEdit::Latitude(_) => "North Latitude",
            SunEdit::Longitude(_) => "East Longitude",
            SunEdit::TimeZone(_) => "Time Zone",
        }
    }
}

impl PhysicalSun {
    /// Apply one row; invalid values leave everything as it was.
    pub fn apply(&mut self, edit: &SunEdit) -> Result<()> {
        let mut next = self.clone();
        match *edit {
            SunEdit::Enabled(on) => next.enabled = on,
            SunEdit::Time(t) => next.time = t,
            SunEdit::Day(m, d) => (next.month, next.day) = (m, d),
            SunEdit::DaylightSaving(on) => next.daylight_saving = on,
            SunEdit::Haze(h) => next.haze = h,
            SunEdit::Clamp(c) => next.clamp = c,
            SunEdit::NorthOffset(a) => next.north_offset = a,
            SunEdit::Location(i) => {
                let (_, lat, lon, tz) = *LOCATIONS.get(i).ok_or("Unknown location")?;
                (next.latitude, next.longitude, next.time_zone) = (lat, lon, tz);
            }
            SunEdit::Place(lat, lon, tz) => {
                (next.latitude, next.longitude, next.time_zone) = (lat, lon, tz)
            }
            SunEdit::Gamma(g) => next.gamma = g,
            SunEdit::Latitude(v) => next.latitude = v,
            SunEdit::Longitude(v) => next.longitude = v,
            SunEdit::TimeZone(v) => next.time_zone = v,
        }
        next.validate()?;
        *self = next;
        Ok(())
    }
}
