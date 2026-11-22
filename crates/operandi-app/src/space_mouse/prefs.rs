//! Preferences > Input > 3D Mouse, saved per user in
//! `space-mouse-preferences.json`; `pref.value spaceMouse.<field>` reads
//! and sets each field (UIC-164). All defaults are provisional: no spec
//! covers 3D mice yet.
use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

/// Which of the puck's motions move the view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AxisLock {
    #[default]
    All,
    /// Only twist and tilt: orbit (and roll when on).
    RotateOnly,
    /// Only pushes: pan and zoom.
    PanOnly,
}
impl AxisLock {
    pub const ALL: [Self; 3] = [Self::All, Self::RotateOnly, Self::PanOnly];
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "Rotate, Pan and Zoom",
            Self::RotateOnly => "Rotate Only",
            Self::PanOnly => "Pan and Zoom Only",
        }
    }
}

/// Per-motion direction flips.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Invert {
    /// Push left/right.
    pub pan_x: bool,
    /// Lift/press.
    pub pan_y: bool,
    /// Push forward/pull back.
    pub zoom: bool,
    /// Tilt forward/back.
    pub tilt: bool,
    /// Twist.
    pub twist: bool,
    /// Tilt left/right (when Roll is on).
    pub roll: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct SpaceMousePreferences {
    pub(crate) version: u8,
    pub enabled: bool,
    /// Multiplies every motion's speed.
    pub speed: f32,
    /// Percent of full deflection ignored around rest (0–50).
    pub dead_zone: u8,
    pub axes: AxisLock,
    /// Tilting the puck sideways banks the view. Off by default so a
    /// slightly crooked push never tilts the horizon (a mouse orbit never
    /// does either).
    pub roll: bool,
    pub invert: Invert,
    /// The keyboard shortcut each button presses (empty: none).
    pub buttons: Buttons,
}

/// The most buttons a device can have a shortcut for (provisional; the
/// biggest pucks have about 30).
pub const MAX_BUTTONS: usize = 32;

/// A shortcut per button, in the input map's text form ("Cmd+F", "A").
/// Saved as `button1` ... `button32`, so each is `pref.value
/// spaceMouse.buttons.button<n>`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Buttons(pub [String; MAX_BUTTONS]);

impl Buttons {
    /// Button `index`'s shortcut (0-based), if it has one.
    pub fn get(&self, index: usize) -> Option<&str> {
        self.0
            .get(index)
            .map(String::as_str)
            .filter(|s| !s.is_empty())
    }
    /// The number of rows to show: through the last assigned button.
    pub fn assigned_through(&self) -> usize {
        self.0
            .iter()
            .rposition(|s| !s.is_empty())
            .map_or(0, |i| i + 1)
    }
}

impl Serialize for Buttons {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = s.serialize_map(Some(MAX_BUTTONS))?;
        for (i, shortcut) in self.0.iter().enumerate() {
            map.serialize_entry(&format!("button{}", i + 1), shortcut)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for Buttons {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let map = std::collections::BTreeMap::<String, String>::deserialize(d)?;
        let mut buttons = Buttons::default();
        for (key, shortcut) in map {
            let slot = key
                .strip_prefix("button")
                .and_then(|n| n.parse::<usize>().ok())
                .filter(|n| (1..=MAX_BUTTONS).contains(n))
                .ok_or_else(|| serde::de::Error::custom(format!("unknown button \"{key}\"")))?;
            buttons.0[slot - 1] = shortcut;
        }
        Ok(buttons)
    }
}

pub const SPEED_RANGE: std::ops::RangeInclusive<f32> = 0.1..=10.;
pub const DEAD_ZONE_RANGE: std::ops::RangeInclusive<u8> = 0..=50;

impl Default for SpaceMousePreferences {
    fn default() -> Self {
        Self {
            version: 1,
            enabled: true,
            speed: 1.,
            dead_zone: 5,
            axes: AxisLock::All,
            roll: false,
            invert: Invert::default(),
            buttons: Buttons::default(),
        }
    }
}

impl SpaceMousePreferences {
    pub fn valid(&self) -> bool {
        self.version == 1
            && SPEED_RANGE.contains(&self.speed)
            && DEAD_ZONE_RANGE.contains(&self.dead_zone)
            && self
                .buttons
                .0
                .iter()
                .all(|b| b.is_empty() || crate::input::chord(b).is_some())
    }
    pub fn load(path: &Path) -> Self {
        fs::metadata(path)
            .ok()
            .filter(|m| m.len() <= 16_384)
            .and_then(|_| fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
            .filter(Self::valid)
            .unwrap_or_default()
    }
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if !self.valid() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid 3D mouse preference",
            ));
        }
        let bytes = serde_json::to_vec(self).map_err(io::Error::other)?;
        let temp = path.with_extension(format!("json.tmp-{}", std::process::id()));
        crate::resource_write::write_bytes(path, &temp, &bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_round_trip_and_bad_files_fall_back_to_defaults() {
        let dir = std::env::temp_dir().join(format!("op-spacemouse-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("space-mouse-preferences.json");
        let mut p = SpaceMousePreferences {
            speed: 2.5,
            dead_zone: 12,
            axes: AxisLock::RotateOnly,
            ..Default::default()
        };
        p.invert.twist = true;
        p.buttons.0[1] = "Cmd+F".into();
        p.save(&path).unwrap();
        assert_eq!(SpaceMousePreferences::load(&path), p);

        let text = String::from_utf8(fs::read(&path).unwrap()).unwrap();
        assert!(text.contains("\"deadZone\":12"), "{text}");
        assert!(text.contains("\"panX\":false"), "{text}");
        assert!(text.contains("\"button2\":\"Cmd+F\""), "{text}");
        assert_eq!(p.buttons.get(1), Some("Cmd+F"));
        assert_eq!(p.buttons.get(0), None);
        assert_eq!(p.buttons.assigned_through(), 2);

        // A shortcut Operandi can't read makes the file invalid.
        p.buttons.0[0] = "Hyper+Q".into();
        assert!(p.save(&path).is_err());
        p.buttons.0[0].clear();
        fs::write(&path, br#"{"version":1,"buttons":{"button99":"A"}}"#).unwrap();
        assert_eq!(
            SpaceMousePreferences::load(&path),
            SpaceMousePreferences::default()
        );
        p.save(&path).unwrap();

        fs::write(&path, br#"{"version":1,"speed":99}"#).unwrap();
        assert_eq!(
            SpaceMousePreferences::load(&path),
            SpaceMousePreferences::default()
        );
        p.dead_zone = 80;
        assert!(p.save(&path).is_err());
        fs::remove_dir_all(dir).unwrap();
    }
}
