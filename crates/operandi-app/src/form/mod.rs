//! The shared form component (UIC-98, UIC-125): a command or panel lists
//! its rows (label, kind, default, unit, "enabled when…" condition) and this
//! module does all the layout, keys, greying and sizing.
//!
//! The same rows draw in two modes:
//!
//! - dialog mode (`Dialog`, UIC-80..99): one tab, no sections, the form on
//!   the left and the OK / Cancel column on the right, in a small modal
//!   window sized exactly to its contents;
//! - panel mode (`panel::show`, UIC-100..124): tabs, foldable sections and
//!   rows with a status circle, for property panels.
//!
//! Callers never lay out a row by hand. They build `Row`s, keep the
//! `Values`, and read the values back when the user presses OK (or, in a
//! panel, when a row reports a change).
pub(crate) mod command_form;
mod dialog;
pub(crate) mod layout;
pub(crate) mod panel;
pub(crate) mod range_labels;
pub(crate) mod rows;
pub(crate) mod status;
mod widgets;

pub(crate) use dialog::{Dialog, Outcome, RowRects};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// How a number is shown and typed (UIC-90: the unit is part of the text).
/// Angle, Percent and Scalar are for the Subdivide dialogs and the panels
/// that move onto the form next.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Unit {
    /// Metres, shown in the user's Distance Units.
    Distance,
    /// Degrees.
    Angle,
    /// A percentage, stored as the number shown (50 = 50 %).
    Percent,
    /// A 0–1 factor, shown as a percentage or a plain number by
    /// Preferences > Input > Factor or Range.
    Factor,
    /// A whole number.
    Count,
    /// A plain number.
    Scalar,
    /// A plain number shown with this unit after it (" px", "×", " K",
    /// " s"); typing the unit is optional.
    Suffix(&'static str),
}

impl Unit {
    /// The scrub step family for the mini-slider arrows.
    pub(crate) fn scrub(self) -> crate::scrub::Kind {
        use crate::scrub::Kind;
        match self {
            Self::Distance => Kind::Distance,
            Self::Angle => Kind::Angle,
            Self::Percent => Kind::Percent,
            Self::Count => Kind::Count,
            Self::Factor | Self::Scalar | Self::Suffix(_) => Kind::Scalar,
        }
    }

    fn factor_as_percent() -> bool {
        crate::value_display::factor_is_percent()
    }

    /// `value` as the field shows it, unit included.
    pub(crate) fn format(self, value: f64) -> String {
        match self {
            Self::Distance => crate::workspace::number_text(value, " m"),
            Self::Angle => crate::workspace::number_text(value, " °"),
            Self::Percent => crate::workspace::number_text(value, " %"),
            Self::Factor if Self::factor_as_percent() => {
                crate::workspace::number_text(value * 100., " %")
            }
            Self::Factor | Self::Scalar => crate::workspace::number_text(value, ""),
            Self::Count => format!("{}", value.round() as i64),
            Self::Suffix(unit) => crate::workspace::number_text(value, unit),
        }
    }

    /// Typed text back to a value (expressions and units allowed).
    pub(crate) fn parse(self, text: &str) -> Result<f64, String> {
        use operandi_core::numeric::{Quantity, parse_quantity};
        let value = match self {
            Self::Distance => parse_quantity(text, Quantity::Distance)?,
            Self::Angle => parse_quantity(text, Quantity::Angle)?,
            Self::Percent => parse_quantity(text, Quantity::Percent)?,
            Self::Factor if Self::factor_as_percent() => {
                parse_quantity(text, Quantity::Percent)? / 100.
            }
            Self::Factor | Self::Scalar => parse_quantity(text, Quantity::Scalar)?,
            Self::Count => parse_quantity(text, Quantity::Scalar)?.round(),
            Self::Suffix(unit) => {
                let text = text.trim();
                let text = text.strip_suffix(unit.trim()).unwrap_or(text);
                parse_quantity(text, Quantity::Scalar)?
            }
        };
        if value.is_finite() {
            Ok(value)
        } else {
            Err("Not a number".into())
        }
    }

    /// One scrub step on the field's own scale, for factors shown as
    /// percentages.
    fn scrub_scale(self) -> f64 {
        if self == Self::Factor && Self::factor_as_percent() {
            0.01
        } else {
            1.
        }
    }
}

/// What kind of field a row has.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Kind {
    /// A number with its unit and limits (UIC-90).
    Number { unit: Unit, min: f64, max: f64 },
    /// A checkbox; the row's label is its text, right of the box (UIC-87).
    Check,
    /// One of a list, in a dropdown (UIC-88).
    Choice(Vec<String>),
    /// One of 2–4 short options as joined segments (UIC-89). Panels show
    /// these as dropdowns (UIC-113).
    Segments(Vec<String>),
    /// Free text (UIC-91). `suggestions` adds a small list of existing
    /// names at the field's right, as a down-triangle.
    Text { suggestions: Vec<String> },
    /// A linear RGB colour.
    Colour,
    /// A full-width button (UIC-116); its click is reported as a change.
    Button,
    /// Plain text across the form area (confirmations, UIC-97).
    Message,
}

/// A row's value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) enum Value {
    Number(f64),
    Bool(bool),
    /// Index into a Choice or Segments list.
    Choice(usize),
    Text(String),
    Colour([f64; 3]),
    /// Buttons and messages have no value.
    None,
}

impl Value {
    fn same_kind(&self, other: &Value) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

/// When a row responds (UIC-92). Rows never appear or disappear; a row
/// whose condition is false greys out in place.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum When {
    Always,
    /// Not available (yet); `Row::tooltip` says why.
    Never,
    /// The checkbox `key` is ticked.
    Checked(&'static str),
    /// The checkbox `key` is not ticked.
    Unchecked(&'static str),
    /// The choice `key` is one of these indices.
    ChoiceIn(&'static str, Vec<usize>),
    All(Vec<When>),
}

impl When {
    pub(crate) fn holds(&self, values: &Values) -> bool {
        match self {
            Self::Always => true,
            Self::Never => false,
            Self::Checked(key) => values.bool(key),
            Self::Unchecked(key) => !values.bool(key),
            Self::ChoiceIn(key, options) => options.contains(&values.choice(key)),
            Self::All(all) => all.iter().all(|w| w.holds(values)),
        }
    }

    /// Why the condition fails with `values` (`None` when it holds). A
    /// `Never` row explains itself in its `tooltip` (UIC-117).
    fn why_not<'a>(
        &self,
        values: &Values,
        find: &dyn Fn(&str) -> Option<&'a Row>,
        tooltip: &str,
    ) -> Option<String> {
        if self.holds(values) {
            return None;
        }
        let label = |key: &str| find(key).map_or_else(|| key.to_owned(), |r| r.label.clone());
        Some(match self {
            Self::Always => return None,
            Self::Never => tooltip.to_owned(),
            Self::Checked(key) => format!("Turn on {} first", label(key)),
            Self::Unchecked(key) => format!("Turn off {} first", label(key)),
            Self::ChoiceIn(key, options) => {
                let names = find(key).and_then(|r| match &r.kind {
                    Kind::Choice(names) | Kind::Segments(names) => Some(names),
                    _ => None,
                });
                let picks: Vec<&str> = options
                    .iter()
                    .filter_map(|&i| names.and_then(|n| n.get(i)).map(String::as_str))
                    .collect();
                if picks.is_empty() {
                    format!("Change {} first", label(key))
                } else {
                    format!("Set {} to {} first", label(key), picks.join(" or "))
                }
            }
            Self::All(all) => return all.iter().find_map(|w| w.why_not(values, find, tooltip)),
        })
    }

    /// The keys this condition reads.
    fn keys(&self, out: &mut Vec<&'static str>) {
        match self {
            Self::Always | Self::Never => {}
            Self::Checked(k) | Self::Unchecked(k) | Self::ChoiceIn(k, _) => out.push(k),
            Self::All(all) => all.iter().for_each(|w| w.keys(out)),
        }
    }
}

/// Who sees a row or a form control (UIC-125's proficiency tag, LAY-86):
/// a panel shows everything up to its level, in this order. Property
/// panels show everything for now; the tag is kept so a later proficiency
/// switch needs no new data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Proficiency {
    #[default]
    Basic,
    Standard,
    Advanced,
}

/// One row of a form.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Row {
    /// Stable key, used for values, remembered values and conditions.
    pub(crate) key: String,
    /// Short Title Case label, no colon (UIC-86).
    pub(crate) label: String,
    pub(crate) kind: Kind,
    pub(crate) default: Value,
    pub(crate) when: When,
    /// Help for the row; shown as its tooltip (UIC-82, UIC-119).
    pub(crate) tooltip: String,
    /// A channel: panels show its status light (UIC-110/112).
    pub(crate) channel: bool,
    pub(crate) proficiency: Proficiency,
    /// Dialogs reopen with the last OK'd value (UIC-95), unless a spec
    /// says the field starts fresh each time.
    pub(crate) remember: bool,
}

impl Row {
    fn new(key: &str, label: &str, kind: Kind, default: Value) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            kind,
            default,
            when: When::Always,
            tooltip: String::new(),
            channel: true,
            proficiency: Proficiency::Basic,
            remember: true,
        }
    }
    pub(crate) fn number(key: &str, label: &str, unit: Unit, default: f64) -> Self {
        Self::new(
            key,
            label,
            Kind::Number {
                unit,
                min: f64::NEG_INFINITY,
                max: f64::INFINITY,
            },
            Value::Number(default),
        )
    }
    pub(crate) fn check(key: &str, label: &str, default: bool) -> Self {
        Self::new(key, label, Kind::Check, Value::Bool(default))
    }
    pub(crate) fn choice(key: &str, label: &str, options: &[&str], default: usize) -> Self {
        Self::new(
            key,
            label,
            Kind::Choice(options.iter().map(|s| s.to_string()).collect()),
            Value::Choice(default),
        )
    }
    pub(crate) fn segments(key: &str, label: &str, options: &[&str], default: usize) -> Self {
        Self::new(
            key,
            label,
            Kind::Segments(options.iter().map(|s| s.to_string()).collect()),
            Value::Choice(default),
        )
    }
    pub(crate) fn text(key: &str, label: &str, default: &str) -> Self {
        Self::new(
            key,
            label,
            Kind::Text {
                suggestions: Vec::new(),
            },
            Value::Text(default.into()),
        )
    }
    pub(crate) fn colour(key: &str, label: &str, default: [f64; 3]) -> Self {
        Self::new(key, label, Kind::Colour, Value::Colour(default))
    }
    pub(crate) fn button(key: &str, label: &str) -> Self {
        let mut row = Self::new(key, label, Kind::Button, Value::None);
        row.channel = false;
        row.remember = false;
        row
    }
    /// Plain text across the form (UIC-97).
    pub(crate) fn message(text: &str) -> Self {
        let mut row = Self::new("message", text, Kind::Message, Value::None);
        row.channel = false;
        row.remember = false;
        row
    }
    pub(crate) fn tip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = tooltip.into();
        self
    }
    pub(crate) fn when(mut self, when: When) -> Self {
        self.when = when;
        self
    }
    /// Lower and upper limits of a number.
    /// The extra height this row's field needs for min/max labels (0 for
    /// all but number rows with closed limits, and when they're off).
    pub(crate) fn label_extra(&self) -> f32 {
        match self.kind {
            Kind::Number { min, max, .. } => range_labels::extra(min, max),
            _ => 0.,
        }
    }
    pub(crate) fn range(mut self, lo: f64, hi: f64) -> Self {
        if let Kind::Number { min, max, .. } = &mut self.kind {
            *min = lo;
            *max = hi;
        }
        self
    }
    /// Existing names listed beside a text field.
    pub(crate) fn suggestions(mut self, names: Vec<String>) -> Self {
        if let Kind::Text { suggestions } = &mut self.kind {
            *suggestions = names;
        }
        self
    }
    /// Starts at its default every time (a spec says so).
    pub(crate) fn fresh(mut self) -> Self {
        self.remember = false;
        self
    }
    /// Not a channel: no status light (UIC-110/112). For the panels that
    /// move onto the form next.
    #[allow(dead_code)]
    pub(crate) fn not_channel(mut self) -> Self {
        self.channel = false;
        self
    }
    #[allow(dead_code)]
    pub(crate) fn advanced(mut self) -> Self {
        self.proficiency = Proficiency::Advanced;
        self
    }
    fn label_in_column(&self) -> bool {
        !matches!(self.kind, Kind::Check | Kind::Message)
    }
}

/// Problems with a list of rows: duplicate keys, a condition naming a
/// later (or missing) row (UIC-93), or a default of the wrong type.
pub(crate) fn check_rows(rows: &[Row]) -> Result<(), String> {
    for (i, row) in rows.iter().enumerate() {
        if row.kind != Kind::Message && rows[..i].iter().any(|r| r.key == row.key) {
            return Err(format!("duplicate row key {}", row.key));
        }
        let mut keys = Vec::new();
        row.when.keys(&mut keys);
        for key in keys {
            if !rows[..i].iter().any(|r| r.key == key) {
                return Err(format!(
                    "{}: its controlling option {key} must sit above it (UIC-93)",
                    row.key
                ));
            }
        }
        // Greyed controls explain themselves (unavailable.rs).
        if row.when == When::Never && row.tooltip.trim().is_empty() {
            return Err(format!(
                "{}: an unavailable row needs a tooltip saying why",
                row.key
            ));
        }
        let fits = match (&row.kind, &row.default) {
            (Kind::Number { .. }, Value::Number(_))
            | (Kind::Check, Value::Bool(_))
            | (Kind::Text { .. }, Value::Text(_))
            | (Kind::Colour, Value::Colour(_))
            | (Kind::Button | Kind::Message, Value::None) => true,
            (Kind::Choice(o) | Kind::Segments(o), Value::Choice(i)) => *i < o.len().max(1),
            _ => false,
        };
        if !fits {
            return Err(format!("{}: default does not fit its kind", row.key));
        }
    }
    Ok(())
}

/// A form's current values, by row key.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Values(pub(crate) BTreeMap<String, Value>);

impl Values {
    /// Every row at its default.
    pub(crate) fn defaults(rows: &[Row]) -> Self {
        Self(
            rows.iter()
                .filter(|r| r.default != Value::None)
                .map(|r| (r.key.clone(), r.default.clone()))
                .collect(),
        )
    }
    pub(crate) fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key)
    }
    pub(crate) fn set(&mut self, key: &str, value: Value) {
        self.0.insert(key.into(), value);
    }
    pub(crate) fn number(&self, key: &str) -> f64 {
        match self.0.get(key) {
            Some(Value::Number(v)) => *v,
            _ => 0.,
        }
    }
    pub(crate) fn bool(&self, key: &str) -> bool {
        matches!(self.0.get(key), Some(Value::Bool(true)))
    }
    pub(crate) fn choice(&self, key: &str) -> usize {
        match self.0.get(key) {
            Some(Value::Choice(i)) => *i,
            _ => 0,
        }
    }
    pub(crate) fn text(&self, key: &str) -> &str {
        match self.0.get(key) {
            Some(Value::Text(t)) => t,
            _ => "",
        }
    }
    pub(crate) fn colour(&self, key: &str) -> [f64; 3] {
        match self.0.get(key) {
            Some(Value::Colour(c)) => *c,
            _ => [0.; 3],
        }
    }
    /// Is `row` enabled with these values (UIC-92)?
    #[cfg(test)]
    pub(crate) fn enabled(&self, row: &Row) -> bool {
        row.when.holds(self)
    }
    /// Why `row` is greyed with these values, naming the controlling row
    /// in `rows` ("Turn on Linear first"); `None` when it is enabled.
    pub(crate) fn why_disabled<'a>(
        &self,
        row: &Row,
        rows: impl IntoIterator<Item = &'a Row> + Clone,
    ) -> Option<String> {
        let find = |key: &str| rows.clone().into_iter().find(|r| r.key == key);
        row.when.why_not(self, &find, &row.tooltip)
    }
}

/// Each command's last OK'd values, kept across restarts in the user's
/// data folder (UIC-95).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Memory {
    version: u8,
    commands: BTreeMap<String, BTreeMap<String, Value>>,
    /// Each form panel's tab and folded sections (UIC-107).
    panels: BTreeMap<String, panel::State>,
}

/// The file in the data folder.
/// In a Choice row's option, this splits it into submenus: "Library ▸
/// Metal ▸ Gold" shows as Gold in a Metal submenu of a Library submenu,
/// and the field shows just "Gold" (MC-05). A filter lists the matches
/// flat, by their whole name.
pub(crate) const SUBMENU: &str = " ▸ ";

pub(crate) const MEMORY_FILE: &str = "dialog-values.json";

impl Memory {
    pub(crate) fn load(path: &Path) -> Self {
        std::fs::metadata(path)
            .ok()
            .filter(|m| m.len() <= 1 << 20)
            .and_then(|_| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
            .unwrap_or_default()
    }
    pub(crate) fn save(&self, path: &Path) -> std::io::Result<()> {
        let bytes = serde_json::to_vec_pretty(&Memory {
            version: 1,
            commands: self.commands.clone(),
            panels: self.panels.clone(),
        })
        .map_err(std::io::Error::other)?;
        let temp = path.with_extension(format!("json.tmp-{}", std::process::id()));
        crate::resource_write::write_bytes(path, &temp, &bytes)
    }
    /// `rows` at their defaults, with this command's remembered values over
    /// them where a remembered value still fits its row.
    pub(crate) fn values(&self, command: &str, rows: &[Row]) -> Values {
        let mut values = Values::defaults(rows);
        let Some(saved) = self.commands.get(command) else {
            return values;
        };
        for row in rows.iter().filter(|r| r.remember) {
            let Some(saved) = saved.get(&row.key) else {
                continue;
            };
            let fits = saved.same_kind(&row.default)
                && match (&row.kind, saved) {
                    (Kind::Choice(o) | Kind::Segments(o), Value::Choice(i)) => *i < o.len(),
                    (Kind::Number { min, max, .. }, Value::Number(v)) => {
                        v.is_finite() && *v >= *min && *v <= *max
                    }
                    _ => true,
                };
            if fits {
                values.set(&row.key, saved.clone());
            }
        }
        values
    }
    /// Panel `id`'s tab and fold state (UIC-107).
    pub(crate) fn panel(&self, id: &str) -> panel::State {
        self.panels.get(id).cloned().unwrap_or_default()
    }
    pub(crate) fn set_panel(&mut self, id: &str, state: panel::State) {
        self.panels.insert(id.into(), state);
    }
    /// Keep `values` for `command`'s rows that remember (UIC-95).
    pub(crate) fn remember(&mut self, command: &str, rows: &[Row], values: &Values) {
        let entry = self.commands.entry(command.into()).or_default();
        for row in rows.iter().filter(|r| r.remember) {
            if let Some(value) = values.get(&row.key) {
                entry.insert(row.key.clone(), value.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<Row> {
        vec![
            Row::choice("method", "Method", &["Faceted", "Smooth", "Catmull"], 0),
            Row::number("fractal", "Fractal", Unit::Percent, 0.).range(0., 100.),
            Row::check("linear", "Linear UVs", false),
            Row::segments("rules", "Boundary Rules", &["Smooth", "Crease"], 0)
                .when(When::ChoiceIn("method", vec![2])),
            Row::text("name", "Name", "Part").fresh(),
            Row::check("weights", "Ignore Weights", false).when(When::All(vec![
                When::ChoiceIn("method", vec![2]),
                When::Checked("linear"),
            ])),
        ]
    }

    #[test]
    fn greyed_rows_name_the_row_that_controls_them() {
        let rows = rows();
        let mut values = Values::defaults(&rows);
        assert_eq!(
            values.why_disabled(&rows[3], &rows).as_deref(),
            Some("Set Method to Catmull first")
        );
        assert_eq!(
            values.why_disabled(&rows[5], &rows).as_deref(),
            Some("Set Method to Catmull first")
        );
        values.set("method", Value::Choice(2));
        assert_eq!(values.why_disabled(&rows[3], &rows), None);
        assert_eq!(
            values.why_disabled(&rows[5], &rows).as_deref(),
            Some("Turn on Linear UVs first")
        );
        let never = Row::check("x", "X", false)
            .when(When::Never)
            .tip("Not built yet");
        assert_eq!(
            values.why_disabled(&never, &rows).as_deref(),
            Some("Not built yet")
        );
        let mut bad = rows.clone();
        bad.push(Row::check("y", "Y", false).when(When::Never));
        assert!(check_rows(&bad).unwrap_err().contains("needs a tooltip"));
    }

    #[test]
    fn enabled_when_follows_the_controlling_rows_live() {
        // UIC-92: greyed, never hidden.
        let rows = rows();
        let mut values = Values::defaults(&rows);
        assert!(!values.enabled(&rows[3]));
        values.set("method", Value::Choice(2));
        assert!(values.enabled(&rows[3]));
        assert!(!values.enabled(&rows[5]));
        values.set("linear", Value::Bool(true));
        assert!(values.enabled(&rows[5]));
        assert!(!When::Never.holds(&values));
        assert!(When::Unchecked("other").holds(&values));
    }

    #[test]
    fn rows_are_checked_for_order_keys_and_defaults() {
        assert_eq!(check_rows(&rows()), Ok(()));
        // UIC-93: the controlling option sits above.
        let mut bad = rows();
        bad.swap(0, 3);
        assert!(check_rows(&bad).unwrap_err().contains("UIC-93"));
        let dup = vec![Row::check("a", "A", false), Row::check("a", "B", true)];
        assert!(check_rows(&dup).is_err());
        let wrong = vec![Row::new("a", "A", Kind::Check, Value::Number(1.))];
        assert!(check_rows(&wrong).is_err());
        let out = vec![Row::choice("a", "A", &["x"], 3)];
        assert!(check_rows(&out).is_err());
    }

    #[test]
    fn remembered_values_reopen_and_survive_a_restart() {
        // UIC-95: first time defaults, then the last OK'd values.
        let rows = rows();
        let mut memory = Memory::default();
        assert_eq!(memory.values("Subdivide", &rows), Values::defaults(&rows));
        let mut values = Values::defaults(&rows);
        values.set("method", Value::Choice(2));
        values.set("fractal", Value::Number(25.));
        values.set("name", Value::Text("Typed".into()));
        memory.remember("Subdivide", &rows, &values);
        let back = memory.values("Subdivide", &rows);
        assert_eq!(back.choice("method"), 2);
        assert_eq!(back.number("fractal"), 25.);
        // A row the spec starts fresh is not remembered.
        assert_eq!(back.text("name"), "Part");
        // Another command keeps its own.
        assert_eq!(memory.values("Other", &rows), Values::defaults(&rows));

        let dir = std::env::temp_dir().join(format!("operandi-form-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(MEMORY_FILE);
        memory.save(&path).unwrap();
        let reloaded = Memory::load(&path);
        assert_eq!(reloaded.values("Subdivide", &rows), back);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stale_remembered_values_fall_back_to_defaults() {
        let rows = rows();
        let mut memory = Memory::default();
        let mut values = Values::defaults(&rows);
        values.set("method", Value::Choice(9));
        values.set("fractal", Value::Number(400.));
        values.set("linear", Value::Text("wrong".into()));
        memory.remember("Subdivide", &rows, &values);
        let back = memory.values("Subdivide", &rows);
        assert_eq!(back, Values::defaults(&rows));
    }

    #[test]
    fn numbers_show_their_unit_and_read_it_back() {
        // UIC-90: value with unit.
        assert_eq!(Unit::Angle.format(45.), "45 °");
        assert_eq!(Unit::Percent.format(12.5), "12.5 %");
        assert_eq!(Unit::Count.format(3.), "3");
        assert_eq!(Unit::Angle.parse("30").unwrap(), 30.);
        assert_eq!(Unit::Count.parse("2.6").unwrap(), 3.);
        assert!(Unit::Scalar.parse("abc").is_err());
    }
}
