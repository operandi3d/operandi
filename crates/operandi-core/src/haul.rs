//! The shared drag layer, "drag controls" in the UI and "hauls" here
//! (specifications/loop-slice-2026-11-15/BEHAVIOR-DRAG-LAYER.md, BIBLE
//! UIC-76): a drag away from every handle changes one of the live tool's
//! values, picked by mouse button and direction.
//!
//! Two parts, both without a window:
//!
//! - [`table`]: which value each button drives per tool, read from
//!   `presets/factory-hauls.toml` (data, so the user can edit it later:
//!   DL-08). A value is the key of a Tool Properties field; the app changes
//!   it through that field, the same route as typing.
//! - [`Tracker`]: one drag's bookkeeping in screen pixels (DL-02): the axis
//!   lock (DL-04), whole-number stepping (DL-05) and the sign (DL-06).
//!
//! The layer sits in the tool pipe after action centre, falloff, symmetry
//! and snapping, and after the tool's handles get first pick (DL-01). Where
//! the app checks for it is in `operandi-app/src/viewport/haul_view.rs`.
use crate::Tool;
use serde::Deserialize;
use std::sync::OnceLock;

/// Provisional feel numbers, all in one place. Jason hasn't tried them at
/// the mouse yet (QUESTIONS.md section C).
pub mod feel {
    /// DL-Q01 (provisional): the first this many pixels of motion pick the
    /// drag's axis; until then nothing changes.
    pub const AXIS_LOCK_PX: f32 = 4.;
    /// DL-Q02 / LS-Q04 (provisional): a whole-number value (count, sides,
    /// segments) changes by one per this many pixels.
    pub const WHOLE_STEP_PX: f32 = 12.;
    /// LS-Q02 (provisional): Loop Slice's left drag moves the current slice
    /// one slider-bar pixel per screen pixel, the same feel as dragging its
    /// triangle on the bar.
    pub const LOOP_SLICE_BAR_PX_PER_PX: f64 = 1.;
    /// Smooth values with no view scale: change per pixel. Distances
    /// follow the view instead (metres under one pixel at the selection,
    /// like the handle drags).
    pub const PERCENT_PER_PX: f64 = 0.5;
    pub const DEGREES_PER_PX: f64 = 0.5;
    pub const SCALAR_PER_PX: f64 = 0.01;
}

const FACTORY_TOML: &str = include_str!("../presets/factory-hauls.toml");

/// A mouse button the layer can claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Button {
    Left,
    Middle,
    Right,
}

/// Which screen direction drives a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    /// Left/right; right is more.
    Sideways,
    /// Up/down; up is more.
    Up,
}

/// One button's two values (field keys), either may be missing.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Pair {
    pub sideways: Option<String>,
    pub up: Option<String>,
}
impl Pair {
    pub fn get(&self, axis: Axis) -> Option<&str> {
        match axis {
            Axis::Sideways => self.sideways.as_deref(),
            Axis::Up => self.up.as_deref(),
        }
    }
    fn is_empty(&self) -> bool {
        self.sideways.is_none() && self.up.is_none()
    }
}

/// One tool's row of the table.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Row {
    /// The editor tool, by its enum name.
    pub tool: String,
    #[serde(default)]
    pub left: Pair,
    #[serde(default)]
    pub middle: Pair,
    #[serde(default)]
    pub right: Pair,
}
impl Row {
    pub fn pair(&self, button: Button) -> &Pair {
        match button {
            Button::Left => &self.left,
            Button::Middle => &self.middle,
            Button::Right => &self.right,
        }
    }
    /// DL-09: the layer claims only buttons with at least one value.
    pub fn claims(&self, button: Button) -> bool {
        !self.pair(button).is_empty()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(default)]
    tool: Vec<Row>,
}

/// Every tool's row, parsed from TOML text; refuses unknown tools and a
/// tool listed twice.
pub fn parse(text: &str) -> std::result::Result<Vec<(Tool, Row)>, String> {
    let file: File = toml::from_str(text).map_err(|e| e.to_string())?;
    let mut rows: Vec<(Tool, Row)> = Vec::new();
    for row in file.tool {
        let Some(tool) = Tool::ALL.into_iter().find(|t| format!("{t:?}") == row.tool) else {
            return Err(format!("hauls: unknown tool {}", row.tool));
        };
        if rows.iter().any(|(t, _)| *t == tool) {
            return Err(format!("hauls: {} is listed twice", row.tool));
        }
        rows.push((tool, row));
    }
    Ok(rows)
}

/// The factory table, read once.
pub fn table() -> &'static [(Tool, Row)] {
    static ROWS: OnceLock<Vec<(Tool, Row)>> = OnceLock::new();
    ROWS.get_or_init(|| parse(FACTORY_TOML).expect("presets/factory-hauls.toml"))
}

/// `tool`'s row, if the table has one.
pub fn row(tool: Tool) -> Option<&'static Row> {
    table().iter().find(|(t, _)| *t == tool).map(|(_, r)| r)
}

/// Whether a drag with `button` belongs to the layer while `tool` is live
/// (DL-09: otherwise it falls through to the tool's own drag).
pub fn claims(tool: Tool, button: Button) -> bool {
    row(tool).is_some_and(|r| r.claims(button))
}

/// How a value changes with distance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rate {
    /// Whole numbers: one step per `feel::WHOLE_STEP_PX` (DL-05).
    Whole,
    /// Smooth values: this much per pixel.
    PerPixel(f64),
}

/// One haul in progress: where it started, and once the motion is clear,
/// which axis it is locked to (DL-04).
#[derive(Clone, Debug, PartialEq)]
pub struct Tracker {
    pub start: [f32; 2],
    pub axis: Option<Axis>,
    /// The pointer has left the dead zone once (a click never does).
    pub dragging: bool,
    /// Which axes have a value; a button with one value locks to it at once.
    sideways: bool,
    up: bool,
}

impl Tracker {
    /// A haul of `pair` starting at screen point `start` (y grows down).
    pub fn new(start: [f32; 2], pair: &Pair) -> Self {
        let (sideways, up) = (pair.sideways.is_some(), pair.up.is_some());
        Self {
            start,
            // Provisional (DL-B01): a button with a single value takes any
            // motion for it, so a drag that starts a little crooked still
            // works. With two values the first clear direction wins.
            axis: match (sideways, up) {
                (true, false) => Some(Axis::Sideways),
                (false, true) => Some(Axis::Up),
                _ => None,
            },
            dragging: false,
            sideways,
            up,
        }
    }

    /// Whether the pointer has moved far enough to count as a drag rather
    /// than a click.
    pub fn moved(&self, at: [f32; 2]) -> bool {
        let (dx, dy) = (at[0] - self.start[0], at[1] - self.start[1]);
        dx.abs().max(dy.abs()) >= feel::AXIS_LOCK_PX
    }

    /// The pointer is at `at`: the locked axis and the signed distance
    /// along it from the press (right and up positive), or `None` while
    /// the pointer is still inside the dead zone. The lock is decided once
    /// and kept for the drag.
    pub fn update(&mut self, at: [f32; 2]) -> Option<(Axis, f32)> {
        let (dx, dy) = (at[0] - self.start[0], -(at[1] - self.start[1]));
        self.dragging |= self.moved(at);
        if !self.dragging {
            return None;
        }
        if self.axis.is_none() {
            self.axis = Some(if dx.abs() >= dy.abs() {
                Axis::Sideways
            } else {
                Axis::Up
            });
        }
        let axis = self.axis?;
        let has = match axis {
            Axis::Sideways => self.sideways,
            Axis::Up => self.up,
        };
        has.then_some((
            axis,
            match axis {
                Axis::Sideways => dx,
                Axis::Up => dy,
            },
        ))
    }
}

/// The change for `distance` signed pixels at `rate`: whole steps count
/// only full steps (toward zero), smooth values scale.
pub fn change(distance: f32, rate: Rate) -> f64 {
    match rate {
        Rate::Whole => (distance / feel::WHOLE_STEP_PX).trunc() as f64,
        Rate::PerPixel(per) => f64::from(distance) * per,
    }
}
