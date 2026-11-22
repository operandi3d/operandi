//! Scaffolding for systems that are designed but not built yet
//! (docs/ARCHITECTURE-CHECK-2026-11-07.md). Only the vocabulary lives here,
//! so new code can start using it; nothing reads these types yet except
//! tests and the defaults below.
use crate::Tool;
use serde::{Deserialize, Serialize};

/// How a tool or mesh operation lives (colour-nav note 02). Direct tools
/// show their properties in the left Tool Properties panel; procedural and
/// schematic operations are items, shown in the right Properties panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ToolKind {
    /// Edits the mesh now, while the tool is active (every tool today).
    #[default]
    Direct,
    /// An operation item in a mesh's procedural stack, re-evaluated.
    Procedural,
    /// A node wired in the schematic.
    Schematic,
}

impl Tool {
    /// Every tool so far is a direct tool; the first procedural mesh
    /// operation will be the first to answer otherwise.
    pub fn kind(self) -> ToolKind {
        ToolKind::Direct
    }
}

/// Form View Options proficiency (colour-nav note 03): which controls a
/// form shows at each level. Every control is Standard unless tagged.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Proficiency {
    Basic,
    #[default]
    Standard,
    Advanced,
}

impl Proficiency {
    /// Whether a control tagged `self` shows in a form set to `level`.
    pub fn shown_at(self, level: Proficiency) -> bool {
        self <= level
    }
}

/// What kind of value a channel holds (colour-nav note 08). Unit kinds
/// share a float value and differ in display and conversion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChannelType {
    Float,
    Distance,
    Angle,
    Percent,
    Integer,
    Bool,
    Enum,
    String,
    Colour,
    Vector,
    Matrix,
}

/// One channel's stored value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ChannelValue {
    Number(f64),
    Bool(bool),
    Text(String),
    Vector([f64; 3]),
}

/// A channel's identity: its item and a stable name ("diffuse.color").
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ChannelId {
    pub item: u64,
    pub name: String,
}

/// Everything a generic UI, link check or script needs to know about a
/// channel without knowing the item kind.
#[derive(Clone, Debug, PartialEq)]
pub struct ChannelDef {
    pub name: &'static str,
    pub kind: ChannelType,
    pub default: ChannelValue,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_direct_and_standard() {
        assert_eq!(Tool::Bevel.kind(), ToolKind::Direct);
        assert_eq!(Proficiency::default(), Proficiency::Standard);
        assert!(Proficiency::Basic.shown_at(Proficiency::Standard));
        assert!(!Proficiency::Advanced.shown_at(Proficiency::Standard));
    }
}
