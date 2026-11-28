//! Data-driven Tool Properties (UIC-98, UIC-125).
//!
//! Each tool declares its panel as data: a `ToolForm` of items, most of
//! them `Field`s (a form `Row` for the label, kind, unit, default and
//! tooltip, plus a `Bind` that reads the value from the tool's parameter
//! struct and writes it back). One renderer draws every tool's panel, and
//! one route confirms its fields, so a layout change happens here once.
//!
//! The flow a tool plugs into through `ToolFields`:
//!
//! - numbers (and words) are draft text, confirmed on Enter, on leaving
//!   the field, by a scrub or an arrow click (`Operandi::number`). On
//!   confirm every pending draft is parsed by its row's unit and written
//!   through its `set`, in key order; the first refusal marks that field
//!   and nothing is published (`form_candidate`).
//! - checkboxes, choices and option buttons change the parameters at once,
//!   together with any pending draft text (`form_option`).
//! - `ToolFields::publish` hands the candidate to the editor (one route for
//!   the panel, the command line and scripts). On success every field is
//!   refreshed from the accepted values, so clamps and linked sizes show.
//! - fields that aren't being edited follow values the session changes
//!   itself (handles, Reset, initialization), once per frame.
//!
//! Rows the specs list but nobody has built are kept as `Item::Unbuilt`:
//! they draw nothing (UIC-117) and stay as the record of what exists.
use super::*;
use crate::form::{Kind, Row, Unit, When};
use crate::unavailable::Gate;
use operandi_core::numeric::Quantity;
use std::rc::Rc;

pub(crate) type Get<P, T> = Rc<dyn Fn(&P) -> T>;
pub(crate) type Set<P, T> = Rc<dyn Fn(&mut P, T) -> std::result::Result<(), String>>;
pub(crate) type Put<P, T> = Rc<dyn Fn(&mut P, T)>;
/// Something a button or option runs on the app.
pub(crate) type Act = Rc<dyn Fn(&mut Operandi)>;
/// A shared component's own drawing (`Item::Custom`).
pub(crate) type DrawOwn = Rc<dyn Fn(&mut Operandi, &mut Ui)>;
/// An option's own command, given the parameters with the change made.
pub(crate) type OwnCommand<P> = Rc<dyn Fn(&mut Operandi, &P)>;

/// How a field reads and writes the tool's parameters.
pub(crate) enum Bind<P> {
    /// Draft text read back by the row's unit; `set` may refuse it.
    Number { get: Get<P, f64>, set: Set<P, f64> },
    /// Draft words (not a number), confirmed like a number.
    Text {
        get: Get<P, String>,
        set: Set<P, String>,
    },
    Check {
        get: Get<P, bool>,
        set: Put<P, bool>,
    },
    /// An index into the row's Choice or Segments list.
    Choice {
        get: Get<P, usize>,
        set: Put<P, usize>,
    },
    /// Read-only text.
    Value(String),
    /// Words kept only in the panel (a Tool State's name), not in the
    /// parameters; buttons read it with `Operandi::draft_text`.
    Draft { default: String },
}

/// One field: its row and how it binds to the parameters.
pub(crate) struct Field<P> {
    pub(crate) row: Row,
    pub(crate) bind: Bind<P>,
    /// The field's text after the number ("" for counts), as the draft shows it.
    pub(crate) suffix: &'static str,
    /// Greyed out (UIC-117: greyed, never hidden) with this tooltip.
    pub(crate) disabled_tip: Option<String>,
    /// Choice entries listed greyed, with why (UIC-117).
    pub(crate) greyed_options: Vec<(usize, String)>,
    /// Choice entries left out of the list: unbuilt (UIC-117). The reasons
    /// are the record of what would make them buildable.
    pub(crate) unbuilt_options: Vec<(usize, String)>,
    /// A weak line under a dropdown's entries.
    pub(crate) popup_note: Option<String>,
    /// A choice's closed text when the value is none of its entries.
    pub(crate) otherwise: Option<String>,
    /// Runs this instead of publishing the parameters (an option with its
    /// own command). It gets the parameters with the change made.
    pub(crate) command: Option<OwnCommand<P>>,
    /// MP-02: shown in the short (Mini Properties) view too.
    pub(crate) mini: bool,
    /// An angle whose bare script number is degrees, not the map's usual
    /// radians (Bend's Angle, BN-46); a query still answers in radians.
    pub(crate) script_degrees: bool,
    /// The field's axis in an X/Y/Z group made by [`vector`]: the group
    /// has one link button on its first row (UIC-56, MN-06b).
    pub(crate) link_axis: Option<usize>,
}

impl<P> Field<P> {
    fn new(row: Row, bind: Bind<P>) -> Self {
        let suffix = match &row.kind {
            Kind::Number { unit, .. } => unit_suffix(*unit),
            _ => "",
        };
        Self {
            // Tool Properties rows show no status light yet (they aren't
            // channels in the panel's sense); `channel()` turns it on.
            row: row.not_channel(),
            bind,
            suffix,
            disabled_tip: None,
            greyed_options: Vec::new(),
            unbuilt_options: Vec::new(),
            popup_note: None,
            otherwise: None,
            command: None,
            mini: false,
            script_degrees: false,
            link_axis: None,
        }
    }
    /// BN-46: `tool.attr <tool> <attr> <n>` reads a bare `n` as degrees.
    pub(crate) fn script_degrees(mut self) -> Self {
        self.script_degrees = true;
        self
    }
    /// MP-02: show this field in Mini Properties as well.
    pub(crate) fn mini(mut self) -> Self {
        self.mini = true;
        self
    }
    pub(crate) fn tip(mut self, tip: impl Into<String>) -> Self {
        self.row = self.row.tip(tip);
        self
    }
    /// Greyed out unless `enabled`, saying `why` on hover.
    pub(crate) fn enabled_if(mut self, enabled: bool, why: impl Into<String>) -> Self {
        if !enabled {
            let why = why.into();
            debug_assert!(
                !why.is_empty(),
                "greyed field {:?} needs a reason",
                self.row.key
            );
            self.row = self.row.when(When::Never);
            self.disabled_tip = Some(why).filter(|w: &String| !w.is_empty());
        }
        self
    }
    /// The draft's unit text, where it differs from the unit's usual one.
    pub(crate) fn suffix(mut self, suffix: &'static str) -> Self {
        self.suffix = suffix;
        self
    }
    pub(crate) fn greyed_option(mut self, index: usize, why: impl Into<String>) -> Self {
        self.greyed_options.push((index, why.into()));
        self
    }
    /// Leaves entry `index` out of the dropdown (UIC-117).
    pub(crate) fn unbuilt_option(mut self, index: usize, why: impl Into<String>) -> Self {
        self.unbuilt_options.push((index, why.into()));
        self
    }
    pub(crate) fn popup_note(mut self, note: impl Into<String>) -> Self {
        self.popup_note = Some(note.into());
        self
    }
    pub(crate) fn otherwise(mut self, text: impl Into<String>) -> Self {
        self.otherwise = Some(text.into());
        self
    }
    pub(crate) fn command(mut self, run: impl Fn(&mut Operandi, &P) + 'static) -> Self {
        self.command = Some(Rc::new(run));
        self
    }
    fn enabled(&self) -> bool {
        self.row.when != When::Never
    }
    /// The row as `form::check_rows` should see it: a field greyed by
    /// `enabled_if` explains itself with that reason.
    #[cfg(test)]
    fn checked_row(&self) -> Row {
        let mut row = self.row.clone();
        if row.tooltip.is_empty()
            && let Some(why) = &self.disabled_tip
        {
            row.tooltip = why.clone();
        }
        row
    }
    /// The text to read: a plain number's own unit ("px", "mm") is
    /// optional, so it's taken off; units the reader knows stay.
    fn strip_suffix<'a>(&self, text: &'a str) -> &'a str {
        let unit = self.suffix.trim();
        if matches!(unit, "" | "m" | "°" | "%") {
            return text;
        }
        let text = text.trim();
        text.strip_suffix(unit).map_or(text, str::trim_end)
    }
    fn quantity(&self) -> Quantity {
        match &self.row.kind {
            Kind::Number { unit, .. } => match unit {
                Unit::Distance => Quantity::Distance,
                Unit::Angle => Quantity::Angle,
                Unit::Percent => Quantity::Percent,
                _ => Quantity::Scalar,
            },
            _ => Quantity::Scalar,
        }
    }
}

/// The draft text after a number of this unit.
fn unit_suffix(unit: Unit) -> &'static str {
    match unit {
        Unit::Distance => " m",
        Unit::Angle => "°",
        Unit::Percent => " %",
        Unit::Suffix(s) => s,
        Unit::Factor | Unit::Count | Unit::Scalar => "",
    }
}

/// A button; `enabled` false greys it with `why`.
pub(crate) struct Button<P> {
    pub(crate) label: String,
    pub(crate) tip: String,
    pub(crate) enabled: bool,
    pub(crate) why: String,
    pub(crate) on: ButtonAction<P>,
}

impl<P> Button<P> {
    /// Why it's greyed; `None` when it can be pressed.
    fn why(&self) -> Option<&str> {
        (!self.enabled).then_some(self.why.as_str())
    }
}

pub(crate) enum ButtonAction<P> {
    /// Run on the app (a command).
    Run(Act),
    /// Change the parameters, published like an option.
    Change(Put<P, ()>),
}

impl<P> Button<P> {
    pub(crate) fn run(label: impl Into<String>, run: impl Fn(&mut Operandi) + 'static) -> Self {
        Self {
            label: label.into(),
            tip: String::new(),
            enabled: true,
            why: String::new(),
            on: ButtonAction::Run(Rc::new(run)),
        }
    }
    /// A button running `command`.
    pub(crate) fn command(label: impl Into<String>, command: Command) -> Self {
        Self::run(label, move |app| app.run(command.clone()))
    }
    pub(crate) fn change(label: impl Into<String>, change: impl Fn(&mut P) + 'static) -> Self {
        Self {
            label: label.into(),
            tip: String::new(),
            enabled: true,
            why: String::new(),
            on: ButtonAction::Change(Rc::new(move |p, ()| change(p))),
        }
    }
    pub(crate) fn tip(mut self, tip: impl Into<String>) -> Self {
        self.tip = tip.into();
        self
    }
    pub(crate) fn enabled_if(mut self, enabled: bool, why: impl Into<String>) -> Self {
        self.enabled = enabled;
        self.why = why.into();
        debug_assert!(
            enabled || !self.why.is_empty(),
            "greyed button {:?} needs a reason",
            self.label
        );
        self
    }
}

/// One entry in a tool's panel, top to bottom.
pub(crate) enum Item<P> {
    Field(Box<Field<P>>),
    /// The tool's name above its rows.
    Title(String),
    /// A plain label over a group of rows.
    Heading(String),
    Separator,
    /// Weak wrapped help text.
    Note(String),
    /// An error line.
    Error(String),
    /// A foldable section (UIC-105..107).
    Section {
        name: &'static str,
        folded: bool,
        items: Vec<Item<P>>,
    },
    /// Buttons side by side.
    Buttons(Vec<Button<P>>),
    /// One button across the panel.
    Wide(Button<P>),
    /// A row the specs list but that isn't built: drawn nowhere (UIC-117).
    /// The fields are the record of the option and why it isn't built.
    #[allow(dead_code)]
    Unbuilt {
        label: String,
        value: String,
        why: String,
    },
    /// A dropdown whose entries each run something (Tool States, profile
    /// pickers): `shown` is the closed text.
    Menu {
        key: String,
        label: String,
        shown: String,
        entries: Vec<MenuEntry>,
        /// Greyed with this tooltip when set.
        disabled: Option<String>,
    },
    /// A shared component drawn by its own code (the profile browser).
    Custom(DrawOwn),
}

/// One entry of an `Item::Menu`.
pub(crate) struct MenuEntry {
    pub(crate) name: String,
    pub(crate) selected: bool,
    pub(crate) run: Act,
}

impl MenuEntry {
    pub(crate) fn new(
        name: impl Into<String>,
        selected: bool,
        run: impl Fn(&mut Operandi) + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            selected,
            run: Rc::new(run),
        }
    }
}

/// A tool's whole panel.
pub(crate) struct ToolForm<P> {
    pub(crate) items: Vec<Item<P>>,
}

impl<P> Default for ToolForm<P> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

impl<P> ToolForm<P> {
    pub(crate) fn new() -> Self {
        Self::default()
    }
    pub(crate) fn push(&mut self, item: Item<P>) -> &mut Self {
        self.items.push(item);
        self
    }
    pub(crate) fn field(&mut self, field: Field<P>) -> &mut Self {
        self.push(Item::Field(Box::new(field)))
    }
    pub(crate) fn fields_from(&mut self, fields: Vec<Field<P>>) -> &mut Self {
        for f in fields {
            self.field(f);
        }
        self
    }
    pub(crate) fn title(&mut self, title: impl Into<String>) -> &mut Self {
        self.push(Item::Title(title.into()))
    }
    pub(crate) fn heading(&mut self, text: impl Into<String>) -> &mut Self {
        self.push(Item::Heading(text.into()))
    }
    pub(crate) fn separator(&mut self) -> &mut Self {
        self.push(Item::Separator)
    }
    pub(crate) fn note(&mut self, text: impl Into<String>) -> &mut Self {
        self.push(Item::Note(text.into()))
    }
    pub(crate) fn error(&mut self, text: impl Into<String>) -> &mut Self {
        self.push(Item::Error(text.into()))
    }
    pub(crate) fn buttons(&mut self, buttons: Vec<Button<P>>) -> &mut Self {
        self.push(Item::Buttons(buttons))
    }
    pub(crate) fn wide(&mut self, button: Button<P>) -> &mut Self {
        self.push(Item::Wide(button))
    }
    pub(crate) fn unbuilt(
        &mut self,
        label: impl Into<String>,
        value: impl Into<String>,
        why: impl Into<String>,
    ) -> &mut Self {
        self.push(unbuilt(label, value, why))
    }
    pub(crate) fn section(
        &mut self,
        name: &'static str,
        folded: bool,
        items: Vec<Item<P>>,
    ) -> &mut Self {
        self.push(Item::Section {
            name,
            folded,
            items,
        })
    }
    /// Every field, sections included, top to bottom.
    pub(crate) fn fields(&self) -> Vec<&Field<P>> {
        fn walk<'a, P>(items: &'a [Item<P>], out: &mut Vec<&'a Field<P>>) {
            for item in items {
                match item {
                    Item::Field(f) => out.push(&**f),
                    Item::Section { items, .. } => walk(items, out),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.items, &mut out);
        out
    }
    /// The draft (number and words) fields' keys, outside sections that
    /// start folded (UIC-106: their rows are drawn once opened).
    #[cfg(test)]
    pub(crate) fn draft_keys(&self) -> Vec<String> {
        fn walk<'a, P>(items: &'a [Item<P>], out: &mut Vec<&'a Field<P>>) {
            for item in items {
                match item {
                    Item::Field(f) => out.push(&**f),
                    Item::Section {
                        items,
                        folded: false,
                        ..
                    } => walk(items, out),
                    _ => {}
                }
            }
        }
        let mut fields = Vec::new();
        walk(&self.items, &mut fields);
        fields
            .into_iter()
            .filter(|f| matches!(f.bind, Bind::Number { .. } | Bind::Text { .. }))
            .map(|f| f.row.key.clone())
            .collect()
    }
}

/// Lifting a form over `P` into one over `Q` that holds a `P` (the live
/// tools' extra rows sit beside their parameters).
type Lens<Q, P> = (fn(&Q) -> &P, fn(&mut Q) -> &mut P);

impl<P: 'static> ToolForm<P> {
    /// This form's items, reading and writing the `P` inside a `Q`.
    pub(crate) fn lift<Q: 'static>(self, lens: Lens<Q, P>) -> Vec<Item<Q>> {
        self.items.into_iter().map(|i| lift_item(i, lens)).collect()
    }
}

fn lift_item<P: 'static, Q: 'static>(item: Item<P>, lens: Lens<Q, P>) -> Item<Q> {
    match item {
        Item::Field(f) => Item::Field(Box::new(lift_field(*f, lens))),
        Item::Title(t) => Item::Title(t),
        Item::Heading(t) => Item::Heading(t),
        Item::Separator => Item::Separator,
        Item::Note(t) => Item::Note(t),
        Item::Error(t) => Item::Error(t),
        Item::Section {
            name,
            folded,
            items,
        } => Item::Section {
            name,
            folded,
            items: items.into_iter().map(|i| lift_item(i, lens)).collect(),
        },
        Item::Buttons(b) => Item::Buttons(b.into_iter().map(|b| lift_button(b, lens)).collect()),
        Item::Wide(b) => Item::Wide(lift_button(b, lens)),
        Item::Unbuilt { label, value, why } => Item::Unbuilt { label, value, why },
        Item::Menu {
            key,
            label,
            shown,
            entries,
            disabled,
        } => Item::Menu {
            key,
            label,
            shown,
            entries,
            disabled,
        },
        Item::Custom(c) => Item::Custom(c),
    }
}

fn lift_button<P: 'static, Q: 'static>(b: Button<P>, (_, get_mut): Lens<Q, P>) -> Button<Q> {
    Button {
        label: b.label,
        tip: b.tip,
        enabled: b.enabled,
        why: b.why,
        on: match b.on {
            ButtonAction::Run(a) => ButtonAction::Run(a),
            ButtonAction::Change(c) => {
                ButtonAction::Change(Rc::new(move |q, ()| c(get_mut(q), ())))
            }
        },
    }
}

fn lift_field<P: 'static, Q: 'static>(f: Field<P>, (get, get_mut): Lens<Q, P>) -> Field<Q> {
    macro_rules! lens {
        ($g:expr, $s:expr) => {
            (
                {
                    let g = $g;
                    Rc::new(move |q: &Q| g(get(q))) as Rc<dyn Fn(&Q) -> _>
                },
                {
                    let s = $s;
                    Rc::new(move |q: &mut Q, v| s(get_mut(q), v))
                },
            )
        };
    }
    let bind = match f.bind {
        Bind::Number { get: g, set: s } => {
            let (get, set) = lens!(g, s);
            Bind::Number { get, set }
        }
        Bind::Text { get: g, set: s } => {
            let (get, set) = lens!(g, s);
            Bind::Text { get, set }
        }
        Bind::Check { get: g, set: s } => {
            let (get, set) = lens!(g, s);
            Bind::Check { get, set }
        }
        Bind::Choice { get: g, set: s } => {
            let (get, set) = lens!(g, s);
            Bind::Choice { get, set }
        }
        Bind::Value(t) => Bind::Value(t),
        Bind::Draft { default } => Bind::Draft { default },
    };
    Field {
        row: f.row,
        bind,
        suffix: f.suffix,
        disabled_tip: f.disabled_tip,
        greyed_options: f.greyed_options,
        unbuilt_options: f.unbuilt_options,
        popup_note: f.popup_note,
        otherwise: f.otherwise,
        mini: f.mini,
        script_degrees: f.script_degrees,
        link_axis: f.link_axis,
        command: f
            .command
            .map(|c| -> OwnCommand<Q> { Rc::new(move |app, q| c(app, get(q))) }),
    }
}

// ---- field builders ----

/// A number field: draft text in `unit`, read with `get`, written by `set`
/// (which may refuse the value with a message).
pub(crate) fn number<P>(
    key: &str,
    label: &str,
    unit: Unit,
    get: impl Fn(&P) -> f64 + 'static,
    set: impl Fn(&mut P, f64) -> std::result::Result<(), String> + 'static,
) -> Field<P> {
    Field::new(
        Row::number(key, label, unit, 0.),
        Bind::Number {
            get: Rc::new(get),
            set: Rc::new(set),
        },
    )
}

/// A whole-number field: `set` gets the value once it's a whole number in
/// `lo..=hi`; anything else is refused with `refusal`.
pub(crate) fn count<P>(
    key: &str,
    label: &str,
    (lo, hi): (f64, f64),
    refusal: &'static str,
    get: impl Fn(&P) -> f64 + 'static,
    set: impl Fn(&mut P, u32) + 'static,
) -> Field<P> {
    let mut field = number(key, label, Unit::Count, get, move |p, v| {
        if !v.is_finite() || v.fract() != 0. || v < lo || v > hi {
            return Err(refusal.into());
        }
        set(p, v as u32);
        Ok(())
    });
    // Its limits, for the min/max labels (form::range_labels).
    field.row = field.row.range(lo, hi);
    field
}

/// Three number fields for a vector: "Position X", then "Y", "Z"
/// (UIC-114), keyed `{key}0`..`{key}2`.
pub(crate) fn vector<P>(
    key: &str,
    label: &str,
    unit: Unit,
    get: impl Fn(&P) -> DVec3 + 'static,
    set: impl Fn(&mut P, usize, f64) -> std::result::Result<(), String> + 'static,
) -> Vec<Field<P>> {
    let get = Rc::new(get);
    let set = Rc::new(set);
    (0..3)
        .map(|axis| {
            let shown = if axis == 0 {
                format!("{label} X")
            } else {
                ["X", "Y", "Z"][axis].to_owned()
            };
            let (get, set) = (get.clone(), set.clone());
            let mut field = number(
                &format!("{key}{axis}"),
                &shown,
                unit,
                move |p| get(p)[axis],
                move |p, v| set(p, axis, v),
            );
            field.link_axis = Some(axis);
            field
        })
        .collect()
}

/// A checkbox (UIC-115).
pub(crate) fn check<P>(
    key: &str,
    label: &str,
    get: impl Fn(&P) -> bool + 'static,
    set: impl Fn(&mut P, bool) + 'static,
) -> Field<P> {
    Field::new(
        Row::check(key, label, false),
        Bind::Check {
            get: Rc::new(get),
            set: Rc::new(set),
        },
    )
}

fn choice_bind<P, T: PartialEq + Clone + 'static>(
    options: &[(T, &str)],
    get: impl Fn(&P) -> T + 'static,
    set: impl Fn(&mut P, T) + 'static,
) -> (Vec<String>, Bind<P>) {
    let names = options.iter().map(|(_, n)| (*n).to_owned()).collect();
    let values: Rc<Vec<T>> = Rc::new(options.iter().map(|(v, _)| v.clone()).collect());
    let lookup = values.clone();
    (
        names,
        Bind::Choice {
            get: Rc::new(move |p| {
                let v = get(p);
                lookup.iter().position(|o| *o == v).unwrap_or(usize::MAX)
            }),
            set: Rc::new(move |p, i| {
                if let Some(v) = values.get(i) {
                    set(p, v.clone());
                }
            }),
        },
    )
}

/// A dropdown of `options` (UIC-88/113).
pub(crate) fn choice<P, T: PartialEq + Clone + 'static>(
    key: &str,
    label: &str,
    options: &[(T, &str)],
    get: impl Fn(&P) -> T + 'static,
    set: impl Fn(&mut P, T) + 'static,
) -> Field<P> {
    let (names, bind) = choice_bind(options, get, set);
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    Field::new(Row::choice(key, label, &refs, 0), bind)
}

/// Joined segments (UIC-89; Tool Properties only).
pub(crate) fn segments<P, T: PartialEq + Clone + 'static>(
    key: &str,
    label: &str,
    options: &[(T, &str)],
    get: impl Fn(&P) -> T + 'static,
    set: impl Fn(&mut P, T) + 'static,
) -> Field<P> {
    let (names, bind) = choice_bind(options, get, set);
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    Field::new(Row::segments(key, label, &refs, 0), bind)
}

/// X / Y / Z as segments.
pub(crate) fn axis<P>(
    get: impl Fn(&P) -> usize + 'static,
    set: impl Fn(&mut P, usize) + 'static,
) -> Field<P> {
    segments("axis", "Axis", &[(0, "X"), (1, "Y"), (2, "Z")], get, set)
}

/// Words typed into a field, confirmed like a number.
pub(crate) fn text<P>(
    key: &str,
    label: &str,
    get: impl Fn(&P) -> String + 'static,
    set: impl Fn(&mut P, String) + 'static,
) -> Field<P> {
    Field::new(
        Row::text(key, label, ""),
        Bind::Text {
            get: Rc::new(get),
            set: Rc::new(move |p, t| {
                set(p, t);
                Ok(())
            }),
        },
    )
}

/// Read-only text.
pub(crate) fn value<P>(key: &str, label: &str, text: impl Into<String>) -> Field<P> {
    Field::new(Row::text(key, label, ""), Bind::Value(text.into()))
}

/// A words field kept only in the panel (see `Bind::Draft`).
pub(crate) fn draft<P>(key: &str, label: &str, default: &str) -> Field<P> {
    Field::new(
        Row::text(key, label, default),
        Bind::Draft {
            default: default.into(),
        },
    )
}

/// An unbuilt row (left out, UIC-117).
pub(crate) fn unbuilt<P>(
    label: impl Into<String>,
    value: impl Into<String>,
    why: impl Into<String>,
) -> Item<P> {
    Item::Unbuilt {
        label: label.into(),
        value: value.into(),
        why: why.into(),
    }
}

/// A number field for a 0–1 fraction shown as a percentage: the field
/// reads `get × 100` with `suffix`; `set` gets the typed percentage.
pub(crate) fn percent<P>(
    key: &str,
    label: &str,
    suffix: &'static str,
    get: impl Fn(&P) -> f64 + 'static,
    set: impl Fn(&mut P, f64) -> std::result::Result<(), String> + 'static,
) -> Field<P> {
    number(key, label, Unit::Percent, move |p| get(p) * 100., set).suffix(suffix)
}

/// A dropdown showing one fixed entry, with why it can't change.
pub(crate) fn fixed<P>(label: &str, shown: &str, why: &str) -> Field<P> {
    let shown = shown.to_owned();
    choice(
        &format!("fixed-{label}"),
        label,
        &[((), shown.as_str())],
        |_| (),
        |_, ()| {},
    )
    .tip(why)
    .popup_note(why)
}

/// Turns a slice of `Field`s into `Item`s (for sections).
#[cfg(test)]
pub(crate) fn items<P>(fields: Vec<Field<P>>) -> Vec<Item<P>> {
    fields
        .into_iter()
        .map(|f| Item::Field(Box::new(f)))
        .collect()
}

/// A tool whose Tool Properties come from a `ToolForm`.
pub(crate) trait ToolFields {
    /// The parameters the panel edits (a copy; publishing writes it back).
    type P: Clone + 'static;
    /// The parameters, while the tool is active.
    fn params(app: &Operandi) -> Option<Self::P>;
    /// The tool's panel for these parameters.
    fn form(app: &Operandi, p: &Self::P) -> ToolForm<Self::P>;
    /// Hands a candidate to the editor: Ok when it took it.
    fn publish(app: &mut Operandi, p: Self::P) -> std::result::Result<(), String>;
    /// After a publish was accepted (status text and the like).
    fn published(_app: &mut Operandi) {}
    /// Nothing pending: may the tool be dropped or applied?
    fn settled(app: &mut Operandi) -> bool {
        app.field_errors.is_empty()
    }
    /// Where an option's refused publish is marked, when no draft changed.
    fn option_error_key(_app: &Operandi, _error: &str) -> Option<String> {
        None
    }
    /// The message for unreadable draft text (kept on the field and shown).
    fn parse_error(_key: &str, _label: &str, error: String) -> String {
        error
    }
    /// The status text for a refused field (the field keeps `error`).
    fn refusal_status(error: &str) -> String {
        error.into()
    }
    /// The status text when the editor refused a publish; `key` is the
    /// last changed field ("" for an option).
    fn publish_status(_key: &str, error: &str) -> String {
        Self::refusal_status(error)
    }
    /// After an option or option button was published (or refused).
    fn after_option(_app: &mut Operandi) {}
    /// Once a frame before the panel is drawn: refresh caches the form
    /// reads (Bridge's Apply check).
    fn prepare(_app: &mut Operandi) {}
}

/// A changed option and the command it runs instead of publishing, if any.
type PickedOption<P> = (Put<P, ()>, Option<OwnCommand<P>>);

/// What a frame's drawing asked for.
struct Outcome<P> {
    options: Vec<PickedOption<P>>,
    actions: Vec<Act>,
}

impl<P> Default for Outcome<P> {
    fn default() -> Self {
        Self {
            options: Vec::new(),
            actions: Vec::new(),
        }
    }
}

impl Operandi {
    /// The tool's parameters with every pending draft applied, in key order,
    /// and the keys that changed. The first refusal is returned as
    /// (key, message).
    pub(super) fn form_candidate<T: ToolFields>(
        &self,
    ) -> std::result::Result<(T::P, Vec<String>), (String, String)> {
        let mut p =
            T::params(self).ok_or_else(|| (String::new(), "The tool isn't active".to_string()))?;
        let form = T::form(self, &p);
        let fields = form.fields();
        // An errored field is retried even when its text equals the last
        // confirmed value (recovery after a refused edit).
        let pending: Vec<(String, String)> = self
            .fields
            .iter()
            .filter(|&(key, text)| {
                self.confirmed_fields.get(key) != Some(text) || self.field_errors.contains_key(key)
            })
            .map(|(k, t)| (k.clone(), t.clone()))
            .collect();
        let mut changed = Vec::new();
        for (key, text) in pending {
            let Some(field) = fields.iter().find(|f| f.row.key == key) else {
                continue;
            };
            match &field.bind {
                Bind::Number { set, .. } => {
                    let v = numeric::parse_quantity(field.strip_suffix(&text), field.quantity())
                        .map_err(|e| (key.clone(), T::parse_error(&key, &field.row.label, e)))?;
                    set(&mut p, v).map_err(|e| (key.clone(), e))?;
                }
                Bind::Text { set, .. } => set(&mut p, text).map_err(|e| (key.clone(), e))?,
                _ => continue,
            }
            changed.push(key);
        }
        Ok((p, changed))
    }

    /// Every field of `T`'s panel as `(key, value)`, values as `tool.attr
    /// <tool> <key> ?` answers them (script units). `None` when `T` isn't
    /// the active tool's panel. Read-only rows are left out. The Command
    /// History compares two of these to record changed values (CMD-14).
    pub(super) fn tool_form_values<T: ToolFields>(&self) -> Option<Vec<(String, String)>> {
        let p = T::params(self)?;
        let form = T::form(self, &p);
        Some(
            form.fields()
                .iter()
                .filter(|f| !matches!(f.bind, Bind::Value(_) | Bind::Draft { .. }))
                .map(|f| (f.row.key.clone(), script_value(f, &p)))
                .collect(),
        )
    }

    /// `tool.attr <tool> <attr> <value|?>` for any data-driven panel
    /// (UIC-164/165): finds the active tool's field by key or label,
    /// reads it, or sets it in map units (percent 0–1, radians; `50%` and
    /// `30deg` too) and publishes it through the panel's own route. `None`
    /// when `T` isn't the active tool's panel; `Ok(Some(text))` answers a
    /// query.
    pub(super) fn tool_attr_form<T: ToolFields>(
        &mut self,
        attr: &str,
        value: &str,
    ) -> Option<std::result::Result<Option<String>, String>> {
        use operandi_core::command_names::{script_angle, script_percent};
        let mut p = T::params(self)?;
        let form = T::form(self, &p);
        let fields = form.fields();
        let norm = |s: &str| {
            s.chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
                .to_ascii_lowercase()
        };
        let Some(field) = fields
            .iter()
            .find(|f| norm(&f.row.key) == norm(attr) || norm(&f.row.label) == norm(attr))
        else {
            let names: Vec<String> = fields.iter().map(|f| f.row.key.clone()).collect();
            return Some(Err(format!(
                "This tool has no setting \"{attr}\"; it has {}",
                names.join(", ")
            )));
        };
        let unit = match &field.row.kind {
            Kind::Number { unit, .. } => Some(*unit),
            _ => None,
        };
        let options: Vec<&str> = match &field.row.kind {
            Kind::Choice(o) | Kind::Segments(o) => o.iter().map(String::as_str).collect(),
            _ => Vec::new(),
        };
        if value == "?" {
            return Some(Ok(Some(script_value(field, &p))));
        }
        if let Some(why) = &field.disabled_tip {
            return Some(Err(format!("{} isn't available: {why}", field.row.label)));
        }
        let label = field.row.label.clone();
        let result: std::result::Result<(), String> = match &field.bind {
            Bind::Number { set, .. } => match unit {
                Some(Unit::Percent) => script_percent(value),
                Some(Unit::Angle) if field.script_degrees => {
                    numeric::parse_quantity(field.strip_suffix(value), Quantity::Angle)
                }
                Some(Unit::Angle) => script_angle(value),
                _ => numeric::parse_quantity(field.strip_suffix(value), field.quantity()),
            }
            .and_then(|v| set(&mut p, v)),
            Bind::Text { set, .. } => set(&mut p, value.to_owned()),
            Bind::Check { set, .. } => {
                operandi_core::command_args::boolean(&label, value).map(|b| set(&mut p, b))
            }
            Bind::Choice { set, .. } => {
                operandi_core::command_args::choice(&label, value, &options).and_then(|i| {
                    if field.unbuilt_options.iter().any(|(u, _)| *u == i)
                        || field.greyed_options.iter().any(|(g, _)| *g == i)
                    {
                        Err(format!("{label}: {} isn't available", options[i]))
                    } else {
                        set(&mut p, i);
                        Ok(())
                    }
                })
            }
            Bind::Value(_) | Bind::Draft { .. } => Err(format!("{label} can't be set")),
        };
        if let Err(e) = result {
            return Some(Err(e));
        }
        let key = field.row.key.clone();
        let own = field.command.clone();
        drop(form);
        Some(match own {
            Some(command) => {
                command(self, &p);
                Ok(None)
            }
            None if self.publish_form::<T>(p, vec![key]) => Ok(None),
            None => Err(self.status.clone()),
        })
    }

    /// Publishes `p`; on success every field shows the accepted values.
    pub(super) fn publish_form<T: ToolFields>(&mut self, p: T::P, changed: Vec<String>) -> bool {
        match T::publish(self, p) {
            Ok(()) => {
                self.field_errors.clear();
                self.sync_form::<T>(true);
                T::published(self);
                true
            }
            Err(e) => {
                self.status = T::publish_status(changed.last().map_or("", String::as_str), &e);
                let keys = if changed.is_empty() {
                    T::option_error_key(self, &e).into_iter().collect()
                } else {
                    changed
                };
                for key in keys {
                    self.field_errors.insert(key, e.clone());
                }
                false
            }
        }
    }

    /// Confirms the tool's pending drafts (Enter, leaving a field, a
    /// scrub). False while a field is refused.
    pub(super) fn confirm_form<T: ToolFields>(&mut self) -> bool {
        match self.form_candidate::<T>() {
            Ok((p, changed)) if !changed.is_empty() => self.publish_form::<T>(p, changed),
            Ok(_) => T::settled(self),
            Err((key, e)) => {
                self.status = T::refusal_status(&e);
                self.field_errors.insert(key, e);
                false
            }
        }
    }

    /// Publishes an option change together with any pending draft text.
    fn form_option<T: ToolFields>(
        &mut self,
        change: Put<T::P, ()>,
        command: Option<OwnCommand<T::P>>,
    ) {
        if let Some(command) = command {
            // An option with its own command (Make UVs): it acts on the
            // tool as it is.
            if let Some(mut p) = T::params(self) {
                change(&mut p, ());
                command(self, &p);
            }
            return;
        }
        match self.form_candidate::<T>() {
            Ok((mut p, changed)) => {
                change(&mut p, ());
                self.publish_form::<T>(p, changed);
            }
            Err((key, e)) => {
                self.status = T::refusal_status(&e);
                self.field_errors.insert(key, e);
            }
        }
        T::after_option(self);
    }

    /// Publishes a change to the tool's parameters as an option would
    /// (with any pending draft text).
    #[cfg(test)]
    pub(super) fn form_change<T: ToolFields>(&mut self, change: impl Fn(&mut T::P) + 'static) {
        self.form_option::<T>(Rc::new(move |p, ()| change(p)), None);
    }

    /// Writes the tool's current values into its fields. `all` false only
    /// touches fields nobody is editing (no pending text, no error), so a
    /// value the session changed itself shows without losing typing.
    pub(super) fn sync_form<T: ToolFields>(&mut self, all: bool) {
        let Some(p) = T::params(self) else {
            return;
        };
        let form = T::form(self, &p);
        for field in form.fields() {
            let key = &field.row.key;
            let shown = match &field.bind {
                Bind::Number { get, .. } => workspace::number_text(get(&p), field.suffix),
                Bind::Text { get, .. } => get(&p),
                _ => continue,
            };
            if !all {
                let confirmed = self.confirmed_fields.get(key);
                let idle =
                    self.fields.get(key) == confirmed && !self.field_errors.contains_key(key);
                if !idle || confirmed.is_none_or(|c| *c == shown) {
                    continue;
                }
            }
            self.fields.insert(key.clone(), shown.clone());
            self.confirmed_fields.insert(key.clone(), shown);
        }
    }

    /// A one-line words field (not a number), confirmed on Enter or when
    /// it loses focus, like the numeric fields.
    pub(super) fn text_field(&mut self, ui: &mut Ui, key: &str, label: &str, value: &str) {
        self.confirmed_fields
            .entry(key.into())
            .or_insert_with(|| value.into());
        let text = self
            .fields
            .entry(key.into())
            .or_insert_with(|| value.into());
        let response = option_row::row(ui, label, |ui| {
            ui.add_sized(
                [ui.available_width().max(52.), option_row::ROW_HEIGHT],
                TextEdit::singleline(text).id_salt(key),
            )
        })
        .inner;
        self.field_rects.insert(key.into(), response.rect);
        self.text_editors.insert(response.id);
        if let Some(text) = self.fields.get(key) {
            crate::numeric_tab::register(ui, &response, Some(text));
        }
        if response.has_focus() {
            self.field_focus = Some(response.id);
            self.field_focus_words = true;
        }
        if self.modal_owned_input || self.has_modal() {
            return;
        }
        let editing = response.has_focus() || response.lost_focus();
        if editing && ui.input(|i| i.key_pressed(Key::Escape)) {
            if let Some(text) = self.confirmed_fields.get(key).cloned() {
                self.fields.insert(key.into(), text);
            }
            response.surrender_focus();
            return;
        }
        let enter = editing && ui.input(|i| i.key_pressed(Key::Enter));
        if enter || response.lost_focus() {
            self.confirm_fields();
            if enter {
                response.surrender_focus();
            }
        }
    }

    /// The text of a `Bind::Draft` field (its default until typed in).
    pub(super) fn draft_text(&self, key: &str, default: &str) -> String {
        self.fields
            .get(key)
            .cloned()
            .unwrap_or_else(|| default.into())
    }

    /// Draws tool `T`'s panel and runs what it asked for.
    /// MP-02: whether the active `T` tool marks any field for the short
    /// (Mini Properties) view.
    pub(super) fn tool_has_mini<T: ToolFields>(&self) -> bool {
        T::params(self).is_some_and(|p| T::form(self, &p).fields().iter().any(|f| f.mini))
    }

    /// MP-02: the short view, only the fields marked `mini`, in order
    /// (sections flattened). Edits go the same way as the full panel's.
    pub(super) fn tool_panel_mini<T: ToolFields>(&mut self, ui: &mut Ui) {
        fn keep<P>(items: Vec<Item<P>>, out: &mut Vec<Item<P>>) {
            for item in items {
                match item {
                    Item::Field(f) if f.mini => out.push(Item::Field(f)),
                    Item::Section { items, .. } => keep(items, out),
                    _ => {}
                }
            }
        }
        if T::params(self).is_none() {
            return;
        }
        T::prepare(self);
        self.sync_form::<T>(false);
        let Some(p) = T::params(self) else {
            return;
        };
        let mut items = Vec::new();
        keep(T::form(self, &p).items, &mut items);
        let mut out = Outcome::default();
        self.draw_items(ui, &items, &p, &mut out);
        for (change, command) in out.options {
            self.form_option::<T>(change, command);
        }
        for action in out.actions {
            action(self);
        }
    }

    pub(super) fn tool_panel<T: ToolFields>(&mut self, ui: &mut Ui) {
        if T::params(self).is_none() {
            return;
        }
        T::prepare(self);
        self.sync_form::<T>(false);
        let Some(p) = T::params(self) else {
            return;
        };
        let form = T::form(self, &p);
        let mut out = Outcome::default();
        self.draw_items(ui, &form.items, &p, &mut out);
        for (change, command) in out.options {
            self.form_option::<T>(change, command);
        }
        for action in out.actions {
            action(self);
        }
    }

    fn draw_items<P: 'static>(
        &mut self,
        ui: &mut Ui,
        items: &[Item<P>],
        p: &P,
        out: &mut Outcome<P>,
    ) {
        for item in items {
            // A workbench property control (layouts stage 4) draws one
            // field of the panel: sections open, everything else skipped.
            if let Some(only) = self.workbench.only_field.clone() {
                match item {
                    Item::Field(field) if field.row.key == only => {
                        self.draw_field(ui, field, p, out)
                    }
                    Item::Section { items, .. } => self.draw_items(ui, items, p, out),
                    _ => {}
                }
                continue;
            }
            match item {
                Item::Field(field) => self.draw_field(ui, field, p, out),
                Item::Title(text) => {
                    ui.label(RichText::new(text).color(theme::chrome::active()).strong());
                }
                Item::Heading(text) => {
                    ui.label(text);
                }
                Item::Separator => {
                    ui.separator();
                }
                Item::Note(text) => {
                    ui.weak(text);
                }
                Item::Error(text) => {
                    ui.colored_label(theme::error(), text);
                }
                Item::Section {
                    name,
                    folded,
                    items,
                } => {
                    crate::form::rows::section(ui, name, *folded, |ui| {
                        self.draw_items(ui, items, p, out)
                    });
                }
                Item::Buttons(buttons) => {
                    ui.horizontal(|ui| {
                        for b in buttons {
                            let response = ui.gated(b.why(), egui::Button::new(&b.label));
                            self.button_clicked(response, b, out);
                        }
                    });
                }
                Item::Wide(b) => {
                    let response = ui.gated(
                        b.why(),
                        egui::Button::new(&b.label).min_size(vec2(ui.available_width(), 22.)),
                    );
                    self.button_clicked(response, b, out);
                }
                Item::Unbuilt { .. } => {}
                Item::Menu {
                    key,
                    label,
                    shown,
                    entries,
                    disabled,
                } => {
                    let drawn = ui.gated_ui(disabled.as_deref(), |ui| {
                        option_row::dropdown(ui, key, label, shown, |ui| {
                            let mut pick = None;
                            for e in entries {
                                if ui.selectable_label(e.selected, &e.name).clicked() {
                                    pick = Some(e.run.clone());
                                }
                            }
                            pick
                        })
                        .inner
                    });
                    // Run after drawing, like the buttons.
                    out.actions.extend(drawn.inner);
                }
                // Drawn in place: the component owns its own layout.
                Item::Custom(draw) => draw(self, ui),
            }
        }
    }

    fn button_clicked<P>(&mut self, response: Response, b: &Button<P>, out: &mut Outcome<P>) {
        let response = if b.tip.is_empty() {
            response
        } else {
            response.on_hover_text(&b.tip)
        };
        if response.clicked() {
            match &b.on {
                ButtonAction::Run(act) => out.actions.push(act.clone()),
                ButtonAction::Change(change) => out.options.push((change.clone(), None)),
            }
        }
    }

    fn draw_field<P: 'static>(&mut self, ui: &mut Ui, f: &Field<P>, p: &P, out: &mut Outcome<P>) {
        // Greyed with its `enabled_if` reason, else the row's tooltip (a
        // `When::Never` row explains itself there, UIC-117).
        let why = (!f.enabled()).then(|| f.disabled_tip.as_deref().unwrap_or(&f.row.tooltip));
        let label = f.row.label.as_str();
        let key = f.row.key.as_str();
        let mut picked: Option<Put<P, ()>> = None;
        let drawn = ui.gated_ui(why, |ui| match (&f.bind, &f.row.kind) {
            (Bind::Number { get, .. }, _) => {
                let mut v = get(p);
                let limits = match f.row.kind {
                    crate::form::Kind::Number { min, max, .. } => (min, max),
                    _ => (f64::NEG_INFINITY, f64::INFINITY),
                };
                // UIC-56/57: an X/Y/Z group's link button and Enter keys.
                self.drawing_link = f.link_axis;
                self.number(ui, key, label, &mut v, f.suffix, limits);
                self.drawing_link = None;
            }
            (Bind::Text { get, .. }, _) => {
                self.text_field(ui, key, label, &get(p));
            }
            (Bind::Draft { default }, _) => {
                self.text_field(ui, key, label, default);
            }
            (Bind::Check { get, set }, _) => {
                let mut on = get(p);
                let response = option_row::check(ui, label, &mut on);
                let changed = response.changed();
                if !f.row.tooltip.is_empty() {
                    response.on_hover_text(&f.row.tooltip);
                }
                if changed {
                    let set = set.clone();
                    picked = Some(Rc::new(move |p: &mut P, ()| set(p, on)));
                }
            }
            (Bind::Choice { get, set }, Kind::Segments(names)) => {
                let mut index = get(p);
                let refs: Vec<&str> = names.iter().map(String::as_str).collect();
                if crate::form::rows::segments(ui, label, &mut index, &refs) {
                    let set = set.clone();
                    picked = Some(Rc::new(move |p: &mut P, ()| set(p, index)));
                }
            }
            (Bind::Choice { get, set }, Kind::Choice(names)) => {
                let index = get(p);
                let shown = names
                    .get(index)
                    .cloned()
                    .or_else(|| f.otherwise.clone())
                    .unwrap_or_default();
                let pick = option_row::dropdown(ui, key, label, &shown, |ui| {
                    let mut pick = None;
                    for (i, name) in names.iter().enumerate() {
                        if f.unbuilt_options.iter().any(|(u, _)| *u == i) {
                            continue;
                        }
                        if let Some((_, why)) = f.greyed_options.iter().find(|(g, _)| *g == i) {
                            ui.gated(Some(why), egui::Button::new(name));
                        } else if ui.selectable_label(i == index, name).clicked() && i != index {
                            pick = Some(i);
                        }
                    }
                    if let Some(note) = &f.popup_note {
                        ui.weak(note);
                    }
                    pick
                });
                let response = pick.response;
                if !f.row.tooltip.is_empty() {
                    response.on_hover_text(&f.row.tooltip);
                }
                if let Some(i) = pick.inner {
                    let set = set.clone();
                    picked = Some(Rc::new(move |p: &mut P, ()| set(p, i)));
                }
            }
            (Bind::Value(text), _) => {
                option_row::value(ui, label, text);
            }
            (Bind::Choice { .. }, _) => {}
        });
        // Pick to Add takes a click on the row instead of the field
        // (LAY-82); the row lights green under the pointer.
        if self.workbench_pick_property(ui, key, label, drawn.response.rect) {
            return;
        }
        if let Some(change) = picked {
            out.options.push((change, f.command.clone()));
        }
    }
}

/// A field's value as a script gives it: percents 0-1, angles in radians
/// (`tool.attr ... ?` and the Command History's `tool.attr` lines).
fn script_value<P>(field: &Field<P>, p: &P) -> String {
    let unit = match &field.row.kind {
        Kind::Number { unit, .. } => Some(*unit),
        _ => None,
    };
    match &field.bind {
        Bind::Number { get, .. } => {
            let v = get(p);
            match unit {
                Some(Unit::Percent) => format!("{}", v / 100.),
                Some(Unit::Angle) => format!("{}", v.to_radians()),
                _ => format!("{v}"),
            }
        }
        Bind::Text { get, .. } => get(p),
        Bind::Check { get, .. } => get(p).to_string(),
        Bind::Choice { get, .. } => {
            let i = get(p);
            match &field.row.kind {
                Kind::Choice(o) | Kind::Segments(o) => {
                    o.get(i).map_or_else(|| i.to_string(), |o| o.to_string())
                }
                _ => i.to_string(),
            }
        }
        Bind::Value(text) => text.clone(),
        Bind::Draft { default } => default.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panels::ToolPanel;

    #[derive(Clone, Default)]
    struct P {
        size: DVec3,
        count: u32,
        on: bool,
    }

    fn form() -> ToolForm<P> {
        let mut f = ToolForm::new();
        f.title("Test")
            .fields_from(vector(
                "t_size",
                "Size",
                Unit::Distance,
                |p: &P| p.size,
                |p, a, v| {
                    p.size[a] = v;
                    Ok(())
                },
            ))
            .field(count(
                "t_count",
                "Count",
                (1., 8.),
                "Count is 1 to 8",
                |p: &P| p.count as f64,
                |p, n| p.count = n,
            ))
            .section(
                "More",
                false,
                items(vec![check("t_on", "On", |p: &P| p.on, |p, b| p.on = b)]),
            )
            .unbuilt("Bulge", "0", "not built");
        f
    }

    #[test]
    fn a_form_lists_its_fields_and_draft_keys_in_order() {
        let f = form();
        let labels: Vec<_> = f.fields().iter().map(|f| f.row.label.clone()).collect();
        assert_eq!(labels, ["Size X", "Y", "Z", "Count", "On"]);
        assert_eq!(f.draft_keys(), ["t_size0", "t_size1", "t_size2", "t_count"]);
        // Unbuilt rows are kept as records but aren't fields.
        assert!(f.items.iter().any(|i| matches!(i, Item::Unbuilt { .. })));
        // Every row's key is unique and well-formed (UIC-93).
        let rows: Vec<Row> = f.fields().iter().map(|f| f.checked_row()).collect();
        assert_eq!(crate::form::check_rows(&rows), Ok(()));
    }

    #[test]
    fn binds_read_write_and_refuse() {
        let f = form();
        let mut p = P::default();
        let fields = f.fields();
        let Bind::Number { set, get } = &fields[1].bind else {
            panic!()
        };
        set(&mut p, 2.5).unwrap();
        assert_eq!(get(&p), 2.5);
        assert_eq!(fields[1].suffix, " m");
        assert!(matches!(fields[1].quantity(), Quantity::Distance));
        let Bind::Number { set, .. } = &fields[3].bind else {
            panic!()
        };
        assert_eq!(set(&mut p, 1.5), Err("Count is 1 to 8".into()));
        assert_eq!(set(&mut p, 9.), Err("Count is 1 to 8".into()));
        set(&mut p, 3.).unwrap();
        assert_eq!(p.count, 3);
        assert_eq!(fields[3].suffix, "");
    }

    #[test]
    fn greyed_fields_keep_their_place() {
        // UIC-117: greyed, never hidden.
        let f = check::<P>("x", "X", |p| p.on, |p, b| p.on = b).enabled_if(false, "Why");
        assert!(!f.enabled());
        assert_eq!(f.disabled_tip.as_deref(), Some("Why"));
        let f = check::<P>("x", "X", |p| p.on, |p, b| p.on = b).enabled_if(true, "Why");
        assert!(f.enabled());
    }

    /// A quad on a fresh mesh, with `tool` started from its command name.
    fn app_with(tool: &str) -> Operandi {
        let mut a = Operandi::empty();
        a.editor.dispatch(Command::NewMesh).unwrap();
        a.editor.scene.active_mut().unwrap().mesh = operandi_core::Mesh {
            vertices: vec![
                DVec3::new(-1., -1., 0.),
                DVec3::new(1., -1., 0.),
                DVec3::new(1., 1., 0.),
                DVec3::new(-1., 1., 0.),
            ],
            faces: vec![operandi_core::Face {
                vertices: vec![0, 1, 2, 3],
                uv: None,
                material: "Default".into(),
                smooth: false,
                part: None,
                material_label: None,
            }]
            .into(),
            ..Default::default()
        }
        .into();
        a.editor.history.undo.clear();
        a.execute(tool);
        a
    }

    /// Draws Tool Properties once and returns the drawn draft fields.
    fn draw(a: &mut Operandi) {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1100., 900.))),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.set_max_width(260.);
                    a.parameters(ui);
                });
            },
        );
    }

    /// Every converted tool's rows are well formed, and every draft field it
    /// declares is drawn and reachable in the panel.
    fn declared_rows_are_drawn<T: ToolFields>(tool: &str) {
        let mut a = app_with(tool);
        let p = T::params(&a).unwrap_or_else(|| panic!("{tool} isn't active"));
        let form = T::form(&a, &p);
        let rows: Vec<Row> = form.fields().iter().map(|f| f.checked_row()).collect();
        assert!(!rows.is_empty(), "{tool} declares no fields");
        assert_eq!(crate::form::check_rows(&rows), Ok(()), "{tool}");
        draw(&mut a);
        for key in form.draft_keys() {
            assert!(a.field_rects.contains_key(&key), "{tool}: {key} not drawn");
        }
    }

    #[test]
    fn pilot_tools_declare_their_fields_and_draw_them() {
        use crate::{
            cone_panel::ConeFields, ellipsoid_panel::EllipsoidFields,
            primitive_panel::PrimitiveFields, reduction_panel::ReductionFields,
            shape_panel::ShapeFields, spike_panel::SpikeFields,
        };
        for tool in ["Cube", "Sphere", "Cylinder"] {
            declared_rows_are_drawn::<PrimitiveFields>(tool);
        }
        declared_rows_are_drawn::<ConeFields>("Cone");
        declared_rows_are_drawn::<EllipsoidFields>("Ellipsoid");
        for tool in ["Torus", "Capsule", "Arc", "Tube", "Text"] {
            declared_rows_are_drawn::<ShapeFields>(tool);
        }
        declared_rows_are_drawn::<SpikeFields>("Spike");
        declared_rows_are_drawn::<ReductionFields>("Polygon Reduction");
    }

    /// `T`'s declared rows for the app as it is are well formed, and every
    /// draft field is drawn and reachable.
    fn declared_and_drawn<T: ToolFields>(a: &mut Operandi, what: &str) {
        let p = T::params(a).unwrap_or_else(|| panic!("{what} isn't active"));
        let form = T::form(a, &p);
        let rows: Vec<Row> = form.fields().iter().map(|f| f.checked_row()).collect();
        assert!(!rows.is_empty(), "{what} declares no fields");
        assert_eq!(crate::form::check_rows(&rows), Ok(()), "{what}");
        draw(a);
        for key in form.draft_keys() {
            assert!(a.field_rects.contains_key(&key), "{what}: {key} not drawn");
        }
    }

    /// The panel every live tool shows; adding a tool means adding it here.
    fn live_panel(tool: Tool) -> ToolPanel {
        match tool {
            Tool::Box | Tool::Sphere | Tool::Cylinder => ToolPanel::Primitive,
            Tool::SmoothShift | Tool::Thicken => ToolPanel::Surface,
            Tool::Bevel
            | Tool::TwistBevel
            | Tool::Chamfer
            | Tool::PolyHaul
            | Tool::SketchExtrude
            | Tool::Extrude
            | Tool::Transform
            | Tool::Move
            | Tool::Rotate
            | Tool::Scale
            | Tool::EdgeExtend
            | Tool::EdgeExtrude
            | Tool::EdgeSlide
            | Tool::EdgeCrease
            | Tool::EdgeBevel
            | Tool::EdgeChamfer
            | Tool::EdgeRelax
            | Tool::VertexCreate
            | Tool::VertexBevel
            | Tool::VertexExtrude
            | Tool::VertexMerge
            | Tool::VertexTear
            | Tool::VertexSlide
            | Tool::Tweak
            | Tool::Push
            | Tool::Smooth
            | Tool::Jitter
            | Tool::Quantize
            | Tool::RadialAlign
            | Tool::LinearAlign
            | Tool::Bend
            | Tool::LoopSlice
            | Tool::EdgeSlice
            | Tool::PolygonSlice
            | Tool::CurveSlice
            | Tool::PenSlice
            | Tool::PrimitiveSlice
            | Tool::AxisSlice
            | Tool::Contour
            | Tool::Inset
            | Tool::Shift
            | Tool::Mirror
            | Tool::Lathe
            | Tool::Clone
            | Tool::Array
            | Tool::RadialArray
            | Tool::CurveExtrude
            | Tool::CurveClone
            | Tool::ScatterClone
            | Tool::Symmetrize
            | Tool::SymmetryTool
            | Tool::UvProject
            | Tool::UvViewProject
            | Tool::UvUnwrap
            | Tool::UvRelax
            | Tool::UvPelt
            | Tool::UvPeeler
            | Tool::UvBoxTransform => ToolPanel::Live,
        }
    }

    const LIVE_TOOLS: [Tool; 63] = [
        Tool::Inset,
        Tool::Shift,
        Tool::Box,
        Tool::Sphere,
        Tool::Cylinder,
        Tool::Bevel,
        Tool::TwistBevel,
        Tool::Chamfer,
        Tool::PolyHaul,
        Tool::SketchExtrude,
        Tool::Extrude,
        Tool::SmoothShift,
        Tool::Thicken,
        Tool::Transform,
        Tool::Move,
        Tool::Rotate,
        Tool::Scale,
        Tool::EdgeExtend,
        Tool::EdgeExtrude,
        Tool::EdgeSlide,
        Tool::EdgeCrease,
        Tool::EdgeBevel,
        Tool::EdgeChamfer,
        Tool::EdgeRelax,
        Tool::VertexCreate,
        Tool::VertexBevel,
        Tool::VertexExtrude,
        Tool::VertexMerge,
        Tool::VertexTear,
        Tool::VertexSlide,
        Tool::Tweak,
        Tool::Push,
        Tool::Smooth,
        Tool::Jitter,
        Tool::Quantize,
        Tool::RadialAlign,
        Tool::LinearAlign,
        Tool::Bend,
        Tool::LoopSlice,
        Tool::EdgeSlice,
        Tool::PolygonSlice,
        Tool::CurveSlice,
        Tool::PenSlice,
        Tool::PrimitiveSlice,
        Tool::AxisSlice,
        Tool::Contour,
        Tool::Mirror,
        Tool::Lathe,
        Tool::Clone,
        Tool::Array,
        Tool::RadialArray,
        Tool::CurveExtrude,
        Tool::CurveClone,
        Tool::ScatterClone,
        Tool::Symmetrize,
        Tool::SymmetryTool,
        Tool::UvProject,
        Tool::UvViewProject,
        Tool::UvUnwrap,
        Tool::UvRelax,
        Tool::UvPelt,
        Tool::UvPeeler,
        Tool::UvBoxTransform,
    ];

    /// Every tool kind's Tool Properties come from a declared field list:
    /// each live tool, the session tools, the Create utilities, the draw
    /// tools and Add Loop / Add Point.
    #[test]
    fn every_tool_kind_declares_a_field_list_and_draws_it() {
        // Two quads side by side: edge (1, 4) lies between them.
        let strip = || operandi_core::Mesh {
            vertices: vec![
                DVec3::new(-1., -1., 0.),
                DVec3::new(0., -1., 0.),
                DVec3::new(1., -1., 0.),
                DVec3::new(-1., 1., 0.),
                DVec3::new(0., 1., 0.),
                DVec3::new(1., 1., 0.),
            ],
            faces: [[0, 1, 4, 3], [1, 2, 5, 4]]
                .into_iter()
                .map(|v| operandi_core::Face {
                    vertices: v.to_vec(),
                    uv: None,
                    material: "Default".into(),
                    smooth: false,
                    part: None,
                    material_label: None,
                })
                .collect(),
            ..Default::default()
        };
        for tool in LIVE_TOOLS {
            let mut a = app_with("Move");
            a.run(Command::Drop);
            a.editor.scene.active_mut().unwrap().mesh = strip().into();
            let name = format!("{tool:?}");
            let sel = &mut a.editor.scene.selection;
            if name.starts_with("Edge") {
                sel.mode = Mode::Edge;
                sel.edges = vec![(1, 4)];
            } else if name.starts_with("Vertex") {
                sel.mode = Mode::Vertex;
                sel.vertices = vec![0];
            } else {
                sel.mode = Mode::Polygon;
                sel.faces = vec![0];
            }
            a.activate(tool);
            if a.editor.session.live.is_none() {
                // Some need particular geometry to start (a bevelled strip,
                // a point with three polygons); their rows are checked on
                // a Move session standing in for them.
                assert!(
                    matches!(
                        tool,
                        Tool::Chamfer
                            | Tool::EdgeRelax
                            | Tool::VertexBevel
                            | Tool::VertexExtrude
                            | Tool::VertexTear
                            | Tool::VertexSlide
                            // Needs Symmetry on.
                            | Tool::SymmetryTool
                            // A one-polygon border is a closed loop.
                            | Tool::LinearAlign
                    ),
                    "{tool:?} didn't start: {}",
                    a.status
                );
                a.activate(Tool::Move);
                a.editor.session.live.as_mut().unwrap().tool = tool;
            }
            assert!(
                a.editor
                    .session
                    .live
                    .as_ref()
                    .is_some_and(|l| l.tool == tool),
                "{tool:?} didn't start: {}",
                a.status
            );
            if tool == Tool::VertexCreate {
                // Its Position rows edit the last clicked point.
                a.editor.session.live.as_mut().unwrap().params.sketch_path = vec![DVec3::ZERO];
            }
            let kind = a.tool_panel_kind();
            assert_eq!(kind, live_panel(tool), "{tool:?}");
            let what = format!("{tool:?}");
            match kind {
                ToolPanel::Primitive => {
                    declared_and_drawn::<crate::primitive_panel::PrimitiveFields>(&mut a, &what)
                }
                ToolPanel::Surface => {
                    declared_and_drawn::<crate::smooth_shift_panel::SurfaceFields>(&mut a, &what)
                }
                _ => declared_and_drawn::<crate::live_fields::LiveFields>(&mut a, &what),
            }
        }
        // Move/Rotate/Scale on an item add Negative Scale and the item rows.
        let mut a = app_with("Scale");
        a.editor.scene.selection.mode = Mode::Item;
        declared_and_drawn::<crate::live_fields::LiveFields>(&mut a, "Scale (item)");
        assert!(a.field_rects.contains_key("delta0"));

        for (tool, kind) in [
            ("Spike", ToolPanel::Spike),
            ("Polygon Reduction", ToolPanel::Reduction),
            ("Cone", ToolPanel::Cone),
            ("Ellipsoid", ToolPanel::Ellipsoid),
            ("Torus", ToolPanel::Shape),
        ] {
            assert_eq!(app_with(tool).tool_panel_kind(), kind, "{tool}");
        }
        let mut a = app_with("Move");
        a.run(Command::Drop);
        a.start_interactive_pen();
        assert_eq!(a.tool_panel_kind(), ToolPanel::Pen);
        declared_and_drawn::<crate::pen_interactive_ui::PenFields>(&mut a, "Pen");

        for utility in [
            crate::create_utility::Utility::PasteTool(Default::default()),
            crate::create_utility::Utility::Protractor(Default::default()),
            crate::create_utility::Utility::Dimensions { orient: false },
            crate::create_utility::Utility::QuickAlign(Default::default()),
        ] {
            let mut a = Operandi::empty();
            a.utility = Some(utility);
            assert_eq!(a.tool_panel_kind(), ToolPanel::Utility);
            declared_and_drawn::<crate::create_utility::UtilityFields>(&mut a, utility.label());
        }
        for kind in [
            crate::draw_tools::DrawKind::Curve,
            crate::draw_tools::DrawKind::Bezier,
            crate::draw_tools::DrawKind::Sketch,
            crate::draw_tools::DrawKind::Patch,
        ] {
            let mut a = app_with("Move");
            a.run(Command::Drop);
            a.activate_draw(kind);
            assert_eq!(a.tool_panel_kind(), ToolPanel::Draw, "{}", kind.label());
            declared_and_drawn::<crate::draw_tools::DrawFields>(&mut a, kind.label());
        }
        for tool in [
            crate::edge_panel::EdgeClick::AddLoop,
            crate::edge_panel::EdgeClick::AddPoint,
        ] {
            let mut a = Operandi::empty();
            a.edge_click = Some(tool);
            assert_eq!(a.tool_panel_kind(), ToolPanel::EdgeInsert);
            declared_and_drawn::<crate::edge_panel::EdgeInsertFields>(&mut a, tool.label());
        }
        // Sculpt brushes, with each tip (SC-101).
        for brush in operandi_core::SculptBrush::every() {
            if brush.unavailable().is_some() {
                // Attenuate can't be turned on yet (SCT-Q12).
                continue;
            }
            for tip in [
                operandi_core::sculpt::TipKind::Smooth,
                operandi_core::sculpt::TipKind::Hard,
            ] {
                let mut a = Operandi::empty();
                a.activate_sculpt(brush);
                a.set_sculpt_tip(tip);
                assert_eq!(a.tool_panel_kind(), ToolPanel::Sculpt);
                let what = format!("{} {}", brush.label(), tip.label());
                declared_and_drawn::<crate::sculpt_ui::SculptFields>(&mut a, &what);
            }
        }
    }

    /// Tool Properties rows come only from field lists: no panel draws its
    /// own draft fields or keeps the old per-tool row helpers.
    #[test]
    fn nothing_calls_the_old_row_helpers() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let banned = [
            "self.number(ui",
            "self.text_field(ui",
            ".triple(",
            "fn triple(",
            "unmeasured::",
            "vector_rows",
            "choice_with_unbuilt",
            "option_row::segmented",
            "fn edge_bevel_panel",
            "fn bevel_fields",
            "fn extrude_fields",
            "fn transform_fields",
            "fn tweak_fields",
            "fn slide_extras_fields",
            "fn slice_fields",
            "fn vertex_tool_fields",
            "fn deform_tool_fields",
            "fn deform_falloff_fields",
            "fn item_channel_fields",
            "fn bridge_parameters",
            "fn pen_parameters",
            "fn surface_parameters",
            "fn utility_parameters",
            "fn draw_parameters",
            "fn edge_insert_parameters",
            "fn confirm_bridge_fields",
            "fn confirm_pen_fields",
            "fn confirm_surface_fields",
            "fn confirm_bevel_options",
            "fn slice_field(",
        ];
        for entry in std::fs::read_dir(src).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if !name.ends_with(".rs") || name == "tool_form.rs" {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            for b in banned {
                assert!(!text.contains(b), "{name} uses {b}");
            }
        }
    }

    /// The row kinds the converted tools added to the shared form.
    #[test]
    fn unbuilt_entries_drafts_menus_and_lifts() {
        let f = choice::<P, u32>(
            "k",
            "K",
            &[(0, "A"), (1, "B")],
            |p| p.count,
            |p, n| p.count = n,
        )
        .unbuilt_option(1, "not built");
        assert_eq!(f.unbuilt_options, [(1, "not built".to_string())]);
        // A draft lives only in the panel's fields.
        let mut a = Operandi::empty();
        assert_eq!(a.draft_text("name", "State 1"), "State 1");
        a.fields.insert("name".into(), "Mine".into());
        assert_eq!(a.draft_text("name", "State 1"), "Mine");
        // A lifted form reads and writes the inner parameters.
        #[derive(Clone, Default)]
        struct Q {
            inner: P,
        }
        let mut inner = ToolForm::new();
        inner.field(check("on", "On", |p: &P| p.on, |p, b| p.on = b));
        let items = inner.lift::<Q>((|q| &q.inner, |q| &mut q.inner));
        let Item::Field(f) = &items[0] else { panic!() };
        let Bind::Check { get, set } = &f.bind else {
            panic!()
        };
        let mut q = Q::default();
        set(&mut q, true);
        assert!(q.inner.on && get(&q));
        // A plain number's own unit is optional when typed.
        let f = number::<P>("px", "Length", Unit::Suffix(" px"), |_| 0., |_, _| Ok(()));
        assert_eq!(f.strip_suffix("120 px"), "120");
        let f = number::<P>("m", "Size", Unit::Distance, |_| 0., |_, _| Ok(()));
        assert_eq!(f.strip_suffix("2 m"), "2 m");
    }

    /// Push's Distance and Smooth's numbers now reach the tool from the
    /// panel (their keys were missing from the old confirm route).
    #[test]
    fn push_and_smooth_fields_confirm_into_the_tool() {
        use crate::live_fields::LiveFields;
        let mut a = app_with("Push");
        a.fields.insert("push_distance".into(), "0.2".into());
        assert!(a.confirm_form::<LiveFields>());
        assert_eq!(a.editor.session.live.as_ref().unwrap().params.shift, 0.2);
        let mut a = app_with("Smooth");
        a.fields.insert("smooth_iterations".into(), "7".into());
        a.fields.insert("smooth_strength".into(), "0.5".into());
        assert!(a.confirm_form::<LiveFields>());
        let s = &a.editor.session.live.as_ref().unwrap().params.smooth;
        assert_eq!((s.iterations, s.strength), (7, 0.5));
        a.fields.insert("smooth_iterations".into(), "2.5".into());
        assert!(!a.confirm_form::<LiveFields>());
        assert!(a.field_errors.contains_key("smooth_iterations"));
    }

    #[test]
    fn an_option_publishes_with_pending_text_and_a_refusal_marks_the_field() {
        use crate::cone_panel::ConeFields;
        let mut a = app_with("Cone");
        // Pending text goes with the option, in one publish.
        a.fields.insert("cone_sides".into(), "9".into());
        a.form_change::<ConeFields>(|p| p.make_uvs = !p.make_uvs);
        let p = ConeFields::params(&a).unwrap();
        assert_eq!(p.sides, 9);
        assert_eq!(a.confirmed_fields["cone_sides"], "9");
        // A refused draft marks its field and publishes nothing.
        let before = ConeFields::params(&a).unwrap();
        a.fields.insert("cone_segments".into(), "2.5".into());
        assert!(!a.confirm_form::<ConeFields>());
        assert!(a.field_errors.contains_key("cone_segments"));
        assert_eq!(ConeFields::params(&a).unwrap(), before);
        // Correcting it publishes and clears the error.
        a.fields.insert("cone_segments".into(), "3".into());
        assert!(a.confirm_form::<ConeFields>());
        assert!(a.field_errors.is_empty());
        assert_eq!(ConeFields::params(&a).unwrap().segments, 3);
    }

    #[test]
    fn idle_fields_follow_values_the_session_changes() {
        use crate::spike_panel::SpikeFields;
        let mut a = app_with("Spike");
        draw(&mut a);
        a.execute("spike strength 0.75");
        draw(&mut a);
        assert_eq!(a.fields["spi_strength"], "0.75");
        // Typing isn't overwritten.
        a.fields.insert("spi_strength".into(), "0.5".into());
        a.editor.dispatch(Command::ConfigureSpike(0.25)).unwrap();
        a.sync_form::<SpikeFields>(false);
        assert_eq!(a.fields["spi_strength"], "0.5");
    }
}
