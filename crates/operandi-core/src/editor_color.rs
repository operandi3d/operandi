//! Item editor colours (note 10): a named colour tag on items and shader
//! layers, purely organisational. It marks list rows, filters the Items
//! list and drives pattern selection; it never changes rendering or the
//! viewport.
//!
//! Stored as the name ("red", "lightblue", …), not RGB (IC-B4); an unknown
//! name reads as none. The RGB a row shows comes from the app's theme.
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The fixed palette, in menu order after "(none)".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EditorColor {
    Red,
    Magenta,
    Pink,
    Brown,
    Orange,
    Yellow,
    Green,
    LightGreen,
    Blue,
    LightBlue,
    Cyan,
    Ultramarine,
    Purple,
    LightPurple,
    White,
    Grey,
    DarkGrey,
}

impl EditorColor {
    pub const ALL: [Self; 17] = [
        Self::Red,
        Self::Magenta,
        Self::Pink,
        Self::Brown,
        Self::Orange,
        Self::Yellow,
        Self::Green,
        Self::LightGreen,
        Self::Blue,
        Self::LightBlue,
        Self::Cyan,
        Self::Ultramarine,
        Self::Purple,
        Self::LightPurple,
        Self::White,
        Self::Grey,
        Self::DarkGrey,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Red => "Red",
            Self::Magenta => "Magenta",
            Self::Pink => "Pink",
            Self::Brown => "Brown",
            Self::Orange => "Orange",
            Self::Yellow => "Yellow",
            Self::Green => "Green",
            Self::LightGreen => "Light Green",
            Self::Blue => "Blue",
            Self::LightBlue => "Light Blue",
            Self::Cyan => "Cyan",
            Self::Ultramarine => "Ultramarine",
            Self::Purple => "Purple",
            Self::LightPurple => "Light Purple",
            Self::White => "White",
            Self::Grey => "Grey",
            Self::DarkGrey => "Dark Grey",
        }
    }
    /// The stored and scripted name: lower case, no spaces.
    pub fn key(self) -> &'static str {
        match self {
            Self::Red => "red",
            Self::Magenta => "magenta",
            Self::Pink => "pink",
            Self::Brown => "brown",
            Self::Orange => "orange",
            Self::Yellow => "yellow",
            Self::Green => "green",
            Self::LightGreen => "lightgreen",
            Self::Blue => "blue",
            Self::LightBlue => "lightblue",
            Self::Cyan => "cyan",
            Self::Ultramarine => "ultramarine",
            Self::Purple => "purple",
            Self::LightPurple => "lightpurple",
            Self::White => "white",
            Self::Grey => "grey",
            Self::DarkGrey => "darkgrey",
        }
    }
    /// A name back to a colour: the key or the label, any case, spaces
    /// ignored ("Red", "Light Blue", "lightblue"). `None` for "none" and
    /// anything unknown ("teal", "gray"): the reference's names only.
    pub fn from_name(name: &str) -> Option<Self> {
        let folded: String = name
            .chars()
            .filter(|c| !c.is_whitespace())
            .flat_map(char::to_lowercase)
            .collect();
        Self::ALL.into_iter().find(|c| c.key() == folded)
    }
    /// The stored name of an optional colour ("none" when unset).
    pub fn key_of(color: Option<Self>) -> &'static str {
        color.map_or("none", Self::key)
    }
    /// The menu and command-line label of an optional colour.
    pub fn label_of(color: Option<Self>) -> &'static str {
        color.map_or("(none)", Self::label)
    }
}

/// Serde for `Option<EditorColor>` fields: the name, or nothing; unknown
/// names read as none (IC-B4).
pub mod serde_name {
    use super::*;
    pub fn serialize<S: Serializer>(c: &Option<EditorColor>, s: S) -> Result<S::Ok, S::Error> {
        match c {
            Some(c) => s.serialize_str(c.key()),
            None => s.serialize_none(),
        }
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<EditorColor>, D::Error> {
        let name: Option<String> = Option::deserialize(d)?;
        Ok(name.as_deref().and_then(EditorColor::from_name))
    }
}

/// Serde for a plain `EditorColor` (filter settings).
impl Serialize for EditorColor {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.key())
    }
}
impl<'de> Deserialize<'de> for EditorColor {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let name = String::deserialize(d)?;
        Ok(Self::from_name(&name).unwrap_or(Self::Red))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip_and_unknown_is_none() {
        for c in EditorColor::ALL {
            assert_eq!(EditorColor::from_name(c.key()), Some(c));
            assert_eq!(EditorColor::from_name(c.label()), Some(c));
        }
        assert_eq!(EditorColor::from_name("Red"), Some(EditorColor::Red));
        assert_eq!(EditorColor::from_name("gray"), None);
        assert_eq!(EditorColor::from_name("teal"), None);
        assert_eq!(EditorColor::from_name("none"), None);
        assert_eq!(EditorColor::from_name("chartreuse"), None);
        assert_eq!(EditorColor::ALL.len(), 17);
    }
}
