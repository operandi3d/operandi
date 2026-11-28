//! Command forms (layouts stage 3, LAY-80..87, UIC-210..223): a toolbox,
//! toolbar or menu as data. A form is a JSON file listing, in order,
//! command buttons (each one command line), sub-forms and dividers; the
//! toolbox draws it (`toolbox_form.rs`). Adding a tool button is editing
//! data, never code (UIC-210).
//!
//! - Shipped forms are the files in `assets/forms`, built into the app
//!   (build.rs) and read-only. A user's changed form is a file of the same
//!   format in the data folder's `Layouts/Forms` folder; it wins over the
//!   shipped one, and Revert deletes it (LAY-87, UIC-216). The editor that
//!   writes them is stage 4; the layering, saving and Revert are here.
//! - LAY-84 / UIC-214: a form with a `slot` puts itself at the head or tail
//!   of another form, ordered by `sort` (then id). The target form's file
//!   is never edited.
//! - LAY-85 / UIC-215: a form, or a labelled divider's section, with
//!   `showWhen` shows only while that command line is enabled (UIC-223: the
//!   command's own answer).
//! - LAY-86 / UIC-215: every control has a level (Basic, Standard,
//!   Advanced); a panel shows controls up to its level.
//! - UIC-221: a command line is text, checked against the registry when
//!   drawn; an unknown one shows red and does nothing.
//! - UIC-222: any registry line, alias or `@macro` may be a button; there
//!   are no allow-lists here.
//!
//! - A `space` control is an empty gap of that many points (provisional,
//!   LAY-Q26: the old tabs space their groups apart; a divider draws a
//!   line).
//! - A sub-form whose style is `menu` is a flyout: one button (the
//!   `form` control's label and tooltip) opening a menu of its commands.
//!   It is greyed while none of them can run (UIC-223).
//! - A tooltip's `{shortcut}` is the line's Input Map binding, or
//!   "unassigned", so tooltips follow the editable input map; `{binding}`
//!   is the binding as buttons draw it ("  [P] "), or nothing (batch 4:
//!   the Vertex and Edge tabs' "Label · verb  [key] " tooltips);
//!   `{chord}` is the binding as the input map displays it, or
//!   "Unassigned" (batch 6, the sculpt brushes);
//!   `{variants}` lists the labelled variants ("Option: Set Part...",
//!   batch 5). A button with no tooltip of its own shows its tool's or
//!   tool preset's.
//! - A `heading` control is a plain title over a few buttons; unlike a
//!   labelled divider it doesn't fold (layouts stage 3 batch 3: the
//!   Duplicate tab's Extend / Clone / Array / Duplicate titles).
//! - A sub-form whose style is `flyout` is a tool flyout (batch 3, the
//!   Deform tab): one button wearing one of its commands, the face
//!   (`face` on the `form` control picks the first). A click runs the
//!   face; holding the press or a right-click opens the list, and the
//!   command picked there runs and becomes the face. A corner triangle
//!   marks it. It is greyed only while none of its commands can run.
//!
//! - A sub-form whose style is `hold` is a hold cell (batch 7, the Create
//!   tab's Tube): one button whose click runs its first command; holding
//!   the press (0.8 s) or a right-click opens a menu of all its commands,
//!   each lit from its answer. It wears the icon of its first lit command
//!   (else the first's) and lights while any is on. A corner triangle
//!   marks it. It can share a horizontal row.
//! - A labelled variant's `tag` is a corner tag drawn while it shows
//!   (batch 7: the primitives' "Item" and "Unit"). A horizontal form's
//!   `columns` shares the width as if it had that many controls, so a
//!   short row keeps its cells the grid's size.
//! - `{binding:<line>}` in a tooltip is that line's binding as buttons draw
//!   it (batch 7: Tube's tooltip names Solid Sketch's key, and a variant's
//!   names its button's own); `{andBinding}` is " · " and the binding, or
//!   nothing when unbound.
//!
//! - Layouts stage 4 (LAY-82, UIC-212): a form with `workbench` set is a
//!   workbench, edited where it sits (`workbench.rs`). Its controls carry
//!   a `width` (Normal, Wide, Full Width, in rows of three units), a
//!   `toggle` style, a `popup` face and an `iconFile`, and it may hold
//!   `property` controls: a tool's Tool Properties row by key.
//!
//! The escape hatch: a `widget` control names a custom-drawn widget for
//! what a plain command button can't do yet. Every one is listed, with why,
//! in [`WIDGETS`], so they can be retired.
use super::Proficiency;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The form file format's version. A file of another version is ignored
/// (and the shipped form used), like a layout file (LAY-49).
pub(crate) const FORM_VERSION: u32 = 1;

/// The user's forms folder inside the data folder's Layouts folder.
pub(crate) const USER_FOLDER: &str = "Forms";

/// The escape hatch's widgets (custom-drawn, by id), each with why a
/// command button can't do it yet. Retire one by giving its job a command
/// line, then delete its row and its arm in `toolbox_form.rs`.
#[cfg_attr(not(test), allow(dead_code))] // checked by the shipped-forms test
pub(crate) const WIDGETS: &[(&str, &str)] = &[
    (
        "uv.mapField",
        "UV tab's Map choice: a drop-down of the mesh's maps; forms have no \
         drop-down control yet (a Choice control would retire it)",
    ),
    (
        "select.lassoStyle",
        "Select tab's Lasso Style drop-down: no command sets the lasso \
         style yet and forms have no drop-down control (a Choice control \
         over a lasso-style command would retire it)",
    ),
    (
        "topology.foregroundOpacity",
        "Topology tab's Foreground Opacity field (MT9): runs pref.value \
         display.topology_opacity; forms have no number control yet",
    ),
    (
        "topology.hitSize",
        "Topology tab's Selection Hit Size field (MT10): runs pref.value \
         selection.hit_size; forms have no number control yet",
    ),
    (
        "select.falloffPlaceholder",
        "Select tab's greyed Select Falloff drop-down: Select Falloff isn't \
         built; it goes when the feature and a Choice control exist",
    ),
    (
        "sculpt.pinchSlot",
        "Sculpt tab's Tangent / Center Pinch slot: it wears the remembered \
         pinch brush (a setting) and lists both on hold or right-click; a \
         tool flyout remembers its face per session instead (a flyout whose \
         face follows a setting would retire it)",
    ),
    (
        "sculpt.multiresLevel",
        "Sculpt tab's multiresolution level read-out (Level 2 of 3 · \
         polygons): forms have no read-out control yet (a label whose text \
         comes from a command's answered title would retire it)",
    ),
    (
        "sculpt.weightMap",
        "Sculpt tab's Weight Map drop-down (shared with the falloff panel): \
         forms have no drop-down control yet (a Choice control would retire \
         it)",
    ),
    (
        "sculpt.newWeightMap",
        "Sculpt tab's New Weight Map… button and its in-menu Name / Value \
         form: forms have no inline-form control (a dialog for vertMap.new \
         would retire it)",
    ),
    // Layouts stage 4 (workbench.rs). Not `widget` controls in a form
    // file: the workbench renderer draws them for any form marked
    // `workbench`. Each button among them still runs a registry line and
    // reads its state from it (UIC-223).
    (
        "workbench.tools",
        "Edit Workbench and the edit tools (Pick to Add, Add Divider, Add \
         Divider with Label, Add Tool Chain, Undo Edit, Revert): their lines \
         name the workbench being drawn, which a shipped form can't (a \
         {form} placeholder in form lines would retire it)",
    ),
    (
        "workbench.editItems",
        "A workbench's items while editing: rows (chips along the top) \
         dragged to reorder and right-clicked for their properties; forms \
         have no edit-mode control (the Form Editor, stage 7, edits any form \
         from its own window but not in place, so it stays)",
    ),
    (
        "workbench.property",
        "A property control: the open tool's Tool Properties row drawn by \
         the tool panel itself, filtered to one field; forms have no \
         property control of their own yet",
    ),
    (
        "workbench.topBar",
        "The workbench row along the top: buttons at their unit widths in \
         a slim bar; a horizontal form shares the whole width instead (a \
         toolbar form style with natural widths would retire it)",
    ),
    (
        "select.polygonTagType",
        "Select tab's Polygon Tag Type drop-down: picks run \
         operandi.select.polygonTagType, but forms have no drop-down \
         control (a Choice control would retire it)",
    ),
];

/// How a form lays out its controls (LAY-80).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Style {
    /// One control per row, full width.
    #[default]
    Vertical,
    /// Controls side by side, sharing the width equally.
    Horizontal,
    /// A property sheet (drawn as Vertical in stage 3).
    Sheet,
    /// A menu: as a sub-form, a flyout button; on its own, drawn as
    /// Vertical in stage 3.
    Menu,
    /// A tool flyout (as a sub-form): a button wearing one of its
    /// commands; hold or right-click for the rest. On its own, drawn as
    /// Vertical.
    Flyout,
    /// A hold cell (as a sub-form): a button running its first command;
    /// hold or right-click for a menu of them all. On its own, drawn as
    /// Vertical.
    Hold,
    /// A pie menu (UIC-308): its first eight controls are the slots,
    /// clockwise from the top. A command is a slot running that line, a
    /// sub-form whose style is pie a slot opening that sub-pie (UIC-310),
    /// and a `space` an empty slot that keeps its place (UIC-309). In a
    /// panel it is drawn as Vertical.
    Pie,
}

/// Icon, text or both (LAY-80).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum IconMode {
    Icon,
    Text,
    #[default]
    Both,
}

/// The form's icon size: the toolbox's Form Icon Size scales it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum IconSize {
    Small,
    #[default]
    Normal,
    Large,
}

/// How a command button is drawn. The toolbox's old tabs use three looks;
/// stage 3 keeps each so a converted tab paints exactly as before
/// (provisional, LAY-Q10).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Look {
    /// The toolbox's own flat button: an icon cell (icon mode Icon) or an
    /// icon/text row.
    #[default]
    Toolbox,
    /// A standard framed button with its label (and, in a vertical form,
    /// its shortcut after it).
    Framed,
    /// The toolbox's flat command row even for a tool's line: no border,
    /// the tool's icon unless `icon` names another (the Duplicate tab's
    /// rows; `Toolbox` draws a tool's line as its bordered tool button).
    Flat,
    /// A checkbox with its label, ticked from the command's lit answer;
    /// clicking runs the line (batch 4: the Vertex tab's Show Vertices,
    /// a view option).
    Check,
    /// A standard button at its natural width, its label followed by the
    /// line's shortcut (batch 4: the Vertex tab's Drop Weld).
    Button,
    /// A standard button at its natural width, its label, a gap and the
    /// line's shortcut (batch 5: the Polygon tab's Flip, Spin Quad, Unify...,
    /// Split, Quadruple and Align).
    Natural,
    /// The toolbox's flat button with its label centred and no shortcut,
    /// text-row height (batch 6: the Edit tab's Center X / Y / Z / All).
    Centered,
    /// For a heading: a dim label at the default size (batch 6: the
    /// Sculpt tab's Vertex Map Tools and Tips).
    Weak,
    /// The toolbox's text row at the icon rows' height (batch 7: the
    /// Create tab's Snaps and Precision).
    Tall,
}

/// How much of a workbench row a control takes (LAY-82's alignment). A
/// workbench lays its controls out in rows of three units: Normal takes
/// one, Wide two, Full the whole row. Other forms ignore it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Width {
    #[default]
    Normal,
    Wide,
    Full,
}

/// Lower case with spaces, dashes and underscores gone, for matching a
/// choice typed on a command line ("full width", "toggleButton").
fn squash(name: &str) -> String {
    name.trim()
        .to_ascii_lowercase()
        .replace([' ', '-', '_'], "")
}

impl Width {
    pub(crate) const ALL: [Width; 3] = [Width::Normal, Width::Wide, Width::Full];
    /// Units of a workbench row's three.
    pub(crate) fn units(self) -> usize {
        match self {
            Width::Normal => 1,
            Width::Wide => 2,
            Width::Full => 3,
        }
    }
    pub(crate) fn label(self) -> &'static str {
        match self {
            Width::Normal => "Normal",
            Width::Wide => "Wide",
            Width::Full => "Full Width",
        }
    }
    /// "normal", "wide", "full" or "full width".
    pub(crate) fn named(name: &str) -> Option<Width> {
        let n = squash(name);
        if n == "full" {
            return Some(Width::Full);
        }
        Width::ALL.into_iter().find(|w| squash(w.label()) == n)
    }
}

/// How a workbench button that toggles something is drawn (LAY-82): the
/// form's own look, a checkbox, or a framed toggle button. Both tick or
/// light from the line's answer (UIC-223).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ToggleStyle {
    #[default]
    Default,
    Checkbox,
    ToggleButton,
}

impl ToggleStyle {
    pub(crate) const ALL: [ToggleStyle; 3] = [
        ToggleStyle::Default,
        ToggleStyle::Checkbox,
        ToggleStyle::ToggleButton,
    ];
    pub(crate) fn label(self) -> &'static str {
        match self {
            ToggleStyle::Default => "Default",
            ToggleStyle::Checkbox => "Checkbox",
            ToggleStyle::ToggleButton => "Toggle Button",
        }
    }
    pub(crate) fn named(name: &str) -> Option<ToggleStyle> {
        let n = squash(name);
        ToggleStyle::ALL
            .into_iter()
            .find(|t| squash(t.label()) == n)
    }
}

/// A workbench control's popup face (LAY-82): how a command with a list
/// of choices shows them. Stored and saved; forms have no choice control
/// yet, so every face draws as a button for now (provisional, LAY-Q49).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum PopupFace {
    #[default]
    Default,
    Popup,
    Buttons,
}

impl PopupFace {
    pub(crate) const ALL: [PopupFace; 3] =
        [PopupFace::Default, PopupFace::Popup, PopupFace::Buttons];
    pub(crate) fn label(self) -> &'static str {
        match self {
            PopupFace::Default => "Default",
            PopupFace::Popup => "Pop-up",
            PopupFace::Buttons => "Buttons",
        }
    }
    pub(crate) fn named(name: &str) -> Option<PopupFace> {
        let n = squash(name);
        PopupFace::ALL.into_iter().find(|f| squash(f.label()) == n)
    }
}

/// Where a slot form goes in its target (LAY-84).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum End {
    Head,
    Tail,
}

/// A form's place in another form's head or tail slot (LAY-84).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Slot {
    /// The target form's id.
    pub form: String,
    pub at: End,
    /// Lower first; ties by the slot form's id.
    #[serde(default)]
    pub sort: i32,
}

/// The lines a button runs with a modifier held (UIC-197's Alt/Ctrl
/// variants and Shift). Ctrl is Cmd on macOS. A variant with a label
/// ([`Variant::Shown`]) makes the button show it while its key is held
/// (the MB variant buttons, `button_variants.rs`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Variants {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shift: Option<Variant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alt: Option<Variant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ctrl: Option<Variant>,
}

/// One variant: a line, or a line the button shows while its key is held.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum Variant {
    Line(String),
    /// Shown while its key is held (batch 5: the Polygon tab's Set
    /// Material..., Alt: Set Part...): the button wears this label, lights
    /// and greys from this line and shows this tooltip, and a click runs
    /// what was shown at press. Keys follow the MB buttons: the physical
    /// Control key is Ctrl and Cmd is never a variant key (MB-05).
    Shown {
        line: String,
        label: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        tooltip: String,
        /// A corner tag drawn while it shows (batch 7: "Item", "Unit").
        #[serde(default, skip_serializing_if = "String::is_empty")]
        tag: String,
        /// The icon the button wears while it shows (a rendered icon's
        /// slug, missing-tools-g6 Mesh Paint Instance); empty keeps the
        /// button's own.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        icon: String,
    },
}

impl From<&str> for Variant {
    fn from(line: &str) -> Self {
        Variant::Line(line.to_owned())
    }
}

impl Variant {
    pub(crate) fn line(&self) -> &str {
        match self {
            Variant::Line(line) | Variant::Shown { line, .. } => line,
        }
    }
}

impl Variants {
    fn is_empty(&self) -> bool {
        self.shift.is_none() && self.alt.is_none() && self.ctrl.is_none()
    }
    /// The line a click with these keys runs: Ctrl/Cmd first, then Shift,
    /// then Alt (the island buttons' old order); the plain line otherwise.
    pub(crate) fn pick<'a>(
        &'a self,
        plain: &'a str,
        ctrl: bool,
        shift: bool,
        alt: bool,
    ) -> &'a str {
        let chosen = if ctrl {
            self.ctrl.as_ref()
        } else if shift {
            self.shift.as_ref()
        } else if alt {
            self.alt.as_ref()
        } else {
            None
        };
        chosen.map_or(plain, Variant::line)
    }
    /// Every variant, Shift, Ctrl then Alt (the MB tooltip's order).
    pub(crate) fn all(&self) -> impl Iterator<Item = &Variant> {
        [&self.shift, &self.ctrl, &self.alt].into_iter().flatten()
    }
    /// Some variant has a label: the button shows the held one.
    pub(crate) fn shown(&self) -> bool {
        self.all().any(|v| matches!(v, Variant::Shown { .. }))
    }
}

/// A sub-form: another form by id, or one written in place.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum FormRef {
    Id(String),
    Inline(Box<Form>),
}

/// One control of a form. Exactly one of `command`, `form`, `divider`,
/// `heading`, `widget`, `space`, `property` and `list` is set
/// ([`Form::check`]).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Control {
    /// A command button: the line it runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// A sub-form placed here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form: Option<FormRef>,
    /// A divider; with a non-empty label it starts a collapsible section
    /// running to the next labelled divider or the form's end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub divider: Option<String>,
    /// A plain title that doesn't fold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading: Option<String>,
    /// The escape hatch: a custom-drawn widget by id ([`WIDGETS`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub widget: Option<String>,
    /// An empty gap, in points.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space: Option<f32>,
    /// A tool property placed here (LAY-82's Pick to Add on a Tool
    /// Properties row): `<tool label>/<row key>`. It shows the live row
    /// while that tool is open, greyed otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub property: Option<String>,
    /// A live list (menus as forms): a menu section filled at run time
    /// from the named source (`menu_forms::LISTS`: recent projects, open
    /// scenes, layouts, palettes). The Form Editor shows it as one item
    /// that can be moved or removed but not expanded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list: Option<String>,
    /// The button's text (a section's id for a labelled divider: its
    /// remembered fold state; default the form id and label).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub label: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tooltip: String,
    /// Icon slug (the generator's, `icons.rs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "is_basic")]
    pub level: Proficiency,
    #[serde(default, skip_serializing_if = "is_default_look")]
    pub look: Look,
    #[serde(default, skip_serializing_if = "Variants::is_empty")]
    pub variants: Variants,
    /// For a labelled divider: its section shows only while this line is
    /// enabled (LAY-85).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_when: Option<String>,
    /// A labelled divider's fold-state key (default `<form id>/<label>`);
    /// a heading's or tool flyout's name for tests and the remembered face
    /// (default its label).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Leave the shortcut off the button (half-width rows have no room
    /// for it); the tooltip can still name it.
    #[serde(default, skip_serializing_if = "is_false")]
    pub no_shortcut: bool,
    /// The input-map action whose binding the button draws and its
    /// tooltip's `{binding}` / `{shortcut}` name, when not the line's own
    /// (batch 6: the Edit tab's Slide shows Slide's or Vertex Slide's key).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shortcut: Option<String>,
    /// For a tool flyout: the command its face shows until one is picked.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub face: usize,
    /// Workbench alignment: Normal, Wide or Full Width (LAY-82).
    #[serde(default, skip_serializing_if = "is_normal_width")]
    pub width: Width,
    /// Workbench toggle style (LAY-82).
    #[serde(default, skip_serializing_if = "is_default_toggle")]
    pub toggle: ToggleStyle,
    /// Workbench popup face (LAY-82).
    #[serde(default, skip_serializing_if = "is_default_popup")]
    pub popup: PopupFace,
    /// A picture from disk worn instead of `icon` (LAY-82's custom icon):
    /// a PNG or JPEG path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_file: Option<String>,
    /// Workbench: the label was cleared, so the button shows its icon
    /// alone in a half slot (provisional LAY-Q54, UIC-212). A button with
    /// no icon still shows its name.
    #[serde(default, skip_serializing_if = "is_false")]
    pub no_label: bool,
    /// A menu entry's native key equivalent on the macOS menu bar ("s" is
    /// Cmd+S, "W" Cmd+Shift+W): only File's fixed keys have one; the rest
    /// show their Input Map binding in the title (menus as forms).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_equivalent: Option<String>,
}

fn is_normal_width(w: &Width) -> bool {
    *w == Width::Normal
}
fn is_default_toggle(t: &ToggleStyle) -> bool {
    *t == ToggleStyle::Default
}
fn is_default_popup(f: &PopupFace) -> bool {
    *f == PopupFace::Default
}

fn is_false(b: &bool) -> bool {
    !*b
}
fn is_zero(n: &usize) -> bool {
    *n == 0
}

fn is_basic(p: &Proficiency) -> bool {
    *p == Proficiency::Basic
}
fn is_default_look(l: &Look) -> bool {
    *l == Look::Toolbox
}
fn one() -> u32 {
    FORM_VERSION
}

/// A form (LAY-80).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Form {
    #[serde(default = "one")]
    pub version: u32,
    /// Unique among forms; inline sub-forms may leave it empty.
    #[serde(default)]
    pub id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub label: String,
    #[serde(default)]
    pub style: Style,
    #[serde(default)]
    pub icons: IconMode,
    #[serde(default)]
    pub icon_size: IconSize,
    /// Can be shown in a panel on its own.
    #[serde(default)]
    pub exported: bool,
    /// Shows only while this line is enabled (LAY-85).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_when: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<Slot>,
    /// A horizontal form's width is shared as if it had at least this
    /// many controls (batch 7: Create's Draw row keeps the grid's cells).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub columns: Option<usize>,
    /// "Customizable Workbench" (LAY-82): the form shows an Edit Workbench
    /// button and the user edits it where it sits. The Form Editor's
    /// checkbox sets it (layouts stage 7, LAY-Q53).
    #[serde(default, skip_serializing_if = "is_false")]
    pub workbench: bool,
    #[serde(default)]
    pub controls: Vec<Control>,
}

impl Form {
    /// Reads one form file.
    pub(crate) fn parse(text: &str) -> Result<Form, String> {
        let form: Form = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if form.version != FORM_VERSION {
            return Err(format!(
                "version {} (this build reads {FORM_VERSION})",
                form.version
            ));
        }
        if form.id.trim().is_empty() {
            return Err("a form file needs an id".into());
        }
        form.check()?;
        Ok(form)
    }

    /// Each control is exactly one kind; inline sub-forms too. Command
    /// lines aren't checked here: an unknown one loads and shows red
    /// (UIC-221).
    pub(crate) fn check(&self) -> Result<(), String> {
        for (i, c) in self.controls.iter().enumerate() {
            let kinds = [
                c.command.is_some(),
                c.form.is_some(),
                c.divider.is_some(),
                c.heading.is_some(),
                c.widget.is_some(),
                c.space.is_some(),
                c.property.is_some(),
                c.list.is_some(),
            ]
            .iter()
            .filter(|k| **k)
            .count();
            if kinds != 1 {
                return Err(format!(
                    "{}: control {} must be one of command, form, divider, heading, widget, space, property or list",
                    self.id,
                    i + 1
                ));
            }
            if let Some(FormRef::Inline(inner)) = &c.form {
                inner.check()?;
            }
        }
        Ok(())
    }

    pub(crate) fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a form serializes") + "\n"
    }
}

/// A form's user file name: its id with anything but letters, digits,
/// `.`, `-` and `_` made `_`.
pub(crate) fn file_name(id: &str) -> String {
    let safe: String = id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("{safe}.json")
}

/// Every form the app knows: shipped and the user's, user winning.
#[derive(Clone, Debug, Default)]
pub(crate) struct FormStore {
    shipped: BTreeMap<String, Form>,
    user: BTreeMap<String, Form>,
    /// The user's forms folder, when there is one.
    folder: Option<PathBuf>,
    /// Files that couldn't be read, with why (logged once at start-up).
    pub(crate) problems: Vec<String>,
}

impl FormStore {
    /// The shipped files (name, text) and the user's folder's files. A
    /// shipped file that doesn't read is a build bug and panics in tests
    /// (`shipped_forms_read`); here it is reported and skipped.
    pub(crate) fn read(shipped: &[(&str, &str)], folder: Option<&Path>) -> FormStore {
        let mut store = FormStore {
            folder: folder.map(Path::to_path_buf),
            ..FormStore::default()
        };
        for (name, text) in shipped {
            match Form::parse(text) {
                Ok(form) => {
                    store.shipped.insert(form.id.clone(), form);
                }
                Err(e) => store.problems.push(format!("shipped form {name}: {e}")),
            }
        }
        let Some(folder) = folder else {
            return store;
        };
        let Ok(entries) = std::fs::read_dir(folder) else {
            return store;
        };
        let mut files: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect();
        files.sort();
        for path in files {
            let read = std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|t| Form::parse(&t));
            match read {
                Ok(form) => {
                    store.user.insert(form.id.clone(), form);
                }
                // Ignored: the shipped form shows (LAY-49's rule for layouts).
                Err(e) => store
                    .problems
                    .push(format!("{}: {e}; ignored", path.display())),
            }
        }
        store
    }

    /// The form that shows for `id`: the user's copy, else the shipped one.
    pub(crate) fn get(&self, id: &str) -> Option<&Form> {
        self.user.get(id).or_else(|| self.shipped.get(id))
    }
    pub(crate) fn shipped(&self, id: &str) -> Option<&Form> {
        self.shipped.get(id)
    }
    /// The user has their own copy of `id` (Revert would drop it).
    pub(crate) fn is_changed(&self, id: &str) -> bool {
        self.user.contains_key(id)
    }
    /// Every form id, shipped and the user's.
    pub(crate) fn ids(&self) -> Vec<&str> {
        let mut ids: Vec<&str> = self
            .shipped
            .keys()
            .chain(self.user.keys())
            .map(String::as_str)
            .collect();
        ids.sort();
        ids.dedup();
        ids
    }

    /// Stores the user's copy of a form and writes its file. Shipped forms
    /// are never written (UIC-216); a copy equal to the shipped form is no
    /// change, so its file goes.
    pub(crate) fn save(&mut self, form: Form) -> Result<(), String> {
        form.check()?;
        if self.shipped.get(&form.id) == Some(&form) {
            self.revert(&form.id)?;
            return Ok(());
        }
        if let Some(folder) = &self.folder {
            std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
            std::fs::write(folder.join(file_name(&form.id)), form.to_json())
                .map_err(|e| e.to_string())?;
        }
        self.user.insert(form.id.clone(), form);
        Ok(())
    }

    /// Revert (LAY-87): drops the user's copy of `id` and its file; the
    /// shipped form shows again. `Ok(false)` when there was nothing to drop.
    pub(crate) fn revert(&mut self, id: &str) -> Result<bool, String> {
        let had = self.user.remove(id).is_some();
        if let Some(folder) = &self.folder {
            let path = folder.join(file_name(id));
            if path.exists() {
                std::fs::remove_file(&path).map_err(|e| e.to_string())?;
                return Ok(true);
            }
        }
        Ok(had)
    }

    /// Forms placed in `target`'s head or tail slot, in order (LAY-84).
    pub(crate) fn slotted(&self, target: &str, at: End) -> Vec<&Form> {
        let mut forms: Vec<&Form> = self
            .ids()
            .into_iter()
            .filter_map(|id| self.get(id))
            .filter(|f| {
                f.slot
                    .as_ref()
                    .is_some_and(|s| s.form == target && s.at == at)
            })
            .collect();
        forms.sort_by(|a, b| {
            let sort = |f: &Form| f.slot.as_ref().map_or(0, |s| s.sort);
            sort(a).cmp(&sort(b)).then_with(|| a.id.cmp(&b.id))
        });
        forms
    }
}

/// One thing to draw, from [`plan`]: forms flattened, slots placed,
/// hidden sections and too-advanced controls left out. Owned, so the
/// drawing can run commands (and change the store) while it walks them.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Item {
    /// A command button (`control.command` is set), with its form's icon
    /// mode.
    Button(Box<Control>, IconMode),
    /// Buttons side by side (a horizontal sub-form), sharing the width as
    /// if there were at least `columns` of them. A hold sub-form among
    /// them is written in place (`FormRef::Inline`).
    Row(Vec<Control>, IconMode, usize),
    /// An unlabelled divider.
    Separator,
    /// A labelled divider: a section header. The section's items follow,
    /// up to the matching [`Item::SectionEnd`].
    Section {
        key: String,
        label: String,
    },
    SectionEnd,
    /// The escape hatch.
    Widget(String),
    /// An empty gap, in points.
    Space(f32),
    /// A flyout (a `menu` sub-form): a button with the `form` control's
    /// label and tooltip, opening a menu of these commands.
    Menu {
        label: String,
        tooltip: String,
        /// The menu button's own icon slug (`icon` on the control holding
        /// the menu), drawn left of the label.
        icon: Option<String>,
        controls: Vec<Control>,
    },
    /// A plain title (`key` names it for tests); `weak` draws it dim.
    Heading {
        key: String,
        label: String,
        weak: bool,
    },
    /// A hold cell (a `hold` sub-form, in a vertical form): `tooltip` is
    /// the `form` control's.
    Hold {
        tooltip: String,
        controls: Vec<Control>,
        icons: IconMode,
    },
    /// A tool flyout (a `flyout` sub-form): `key` remembers its face,
    /// `face` is where it starts.
    Flyout {
        key: String,
        face: usize,
        controls: Vec<Control>,
    },
    /// A sub-form id that names no form: shown red, like an unknown line.
    Missing(String),
    /// A tool property (`control.property` is set), workbench only.
    Property(Box<Control>),
}

/// Flattens form `id` into draw items. `enabled` answers whether a
/// command line is enabled now (UIC-223); `level` is the panel's
/// proficiency (LAY-86). Sub-forms nest at most 8 deep, so a form that
/// includes itself stops instead of hanging.
pub(crate) fn plan(
    store: &FormStore,
    id: &str,
    level: Proficiency,
    enabled: &dyn Fn(&str) -> bool,
) -> Vec<Item> {
    let mut out = Vec::new();
    match store.get(id) {
        Some(form) => add_form(store, form, level, enabled, 0, &mut out),
        None => out.push(Item::Missing(id.to_owned())),
    }
    out
}

fn add_form(
    store: &FormStore,
    form: &Form,
    level: Proficiency,
    enabled: &dyn Fn(&str) -> bool,
    depth: usize,
    out: &mut Vec<Item>,
) {
    if depth > 8 || form.show_when.as_deref().is_some_and(|l| !enabled(l)) {
        return;
    }
    for head in store.slotted(&form.id, End::Head) {
        add_form(store, head, level, enabled, depth + 1, out);
    }
    if form.style == Style::Horizontal {
        let row: Vec<Control> = form
            .controls
            .iter()
            .filter(|c| c.level <= level)
            .filter_map(|c| {
                // A widget takes its share of the row too (batch 6: the
                // Sculpt tab's Pinch slot beside Flatten), and so does a
                // hold cell (batch 7: Create's Tube), resolved here.
                if c.command.is_some() || c.widget.is_some() {
                    return Some(c.clone());
                }
                let inner = match c.form.as_ref()? {
                    FormRef::Inline(inner) => inner.as_ref().clone(),
                    FormRef::Id(id) => store.get(id)?.clone(),
                };
                (inner.style == Style::Hold).then(|| Control {
                    form: Some(FormRef::Inline(Box::new(hold_lines(inner, level)))),
                    ..c.clone()
                })
            })
            .collect();
        if !row.is_empty() {
            out.push(Item::Row(row, form.icons, form.columns.unwrap_or(0)));
        }
    } else {
        // Inside a section that is hidden (its showWhen is off, or it is
        // above the level), skip to the next labelled divider.
        let mut hidden = false;
        let mut open_section = false;
        for c in &form.controls {
            if let Some(label) = c.divider.as_deref().filter(|l| !l.trim().is_empty()) {
                if open_section {
                    out.push(Item::SectionEnd);
                    open_section = false;
                }
                hidden = c.level > level || c.show_when.as_deref().is_some_and(|l| !enabled(l));
                if !hidden {
                    let key =
                        c.id.clone()
                            .unwrap_or_else(|| format!("{}/{label}", form.id));
                    out.push(Item::Section {
                        key,
                        label: label.to_owned(),
                    });
                    open_section = true;
                }
                continue;
            }
            if hidden || c.level > level {
                continue;
            }
            if c.command.is_some() {
                out.push(Item::Button(Box::new(c.clone()), form.icons));
            } else if c.divider.is_some() {
                out.push(Item::Separator);
            } else if let Some(label) = &c.heading {
                out.push(Item::Heading {
                    key: c.id.clone().unwrap_or_else(|| label.clone()),
                    label: label.clone(),
                    weak: c.look == Look::Weak,
                });
            } else if let Some(w) = &c.widget {
                out.push(Item::Widget(w.clone()));
            } else if let Some(points) = c.space {
                out.push(Item::Space(points));
            } else if c.property.is_some() {
                out.push(Item::Property(Box::new(c.clone())));
            } else if let Some(sub) = &c.form {
                let inner = match sub {
                    FormRef::Inline(inner) => inner.as_ref(),
                    FormRef::Id(sub_id) => match store.get(sub_id) {
                        Some(inner) => inner,
                        None => {
                            out.push(Item::Missing(sub_id.clone()));
                            continue;
                        }
                    },
                };
                if inner.style == Style::Hold {
                    let inner = hold_lines(inner.clone(), level);
                    if !inner.controls.is_empty() {
                        out.push(Item::Hold {
                            tooltip: c.tooltip.clone(),
                            icons: form.icons,
                            controls: inner.controls,
                        });
                    }
                } else if inner.style == Style::Flyout {
                    if inner.show_when.as_deref().is_some_and(|l| !enabled(l)) {
                        continue;
                    }
                    let controls: Vec<Control> = inner
                        .controls
                        .iter()
                        .filter(|m| m.command.is_some() && m.level <= level)
                        .cloned()
                        .collect();
                    if controls.is_empty() {
                        continue;
                    }
                    let key =
                        c.id.clone()
                            .or_else(|| (!c.label.is_empty()).then(|| c.label.clone()))
                            .unwrap_or_else(|| inner.label.clone());
                    out.push(Item::Flyout {
                        key,
                        face: c.face.min(controls.len() - 1),
                        controls,
                    });
                } else if inner.style == Style::Menu {
                    if inner.show_when.as_deref().is_some_and(|l| !enabled(l)) {
                        continue;
                    }
                    let controls: Vec<Control> = inner
                        .controls
                        .iter()
                        .filter(|m| m.command.is_some() && m.level <= level)
                        .cloned()
                        .collect();
                    let label = if c.label.is_empty() {
                        inner.label.clone()
                    } else {
                        c.label.clone()
                    };
                    out.push(Item::Menu {
                        label,
                        tooltip: c.tooltip.clone(),
                        icon: c.icon.clone(),
                        controls,
                    });
                } else {
                    add_form(store, inner, level, enabled, depth + 1, out);
                }
            }
        }
        if open_section {
            out.push(Item::SectionEnd);
        }
    }
    for tail in store.slotted(&form.id, End::Tail) {
        add_form(store, tail, level, enabled, depth + 1, out);
    }
}

/// A hold sub-form with only its command buttons up to `level`.
fn hold_lines(mut form: Form, level: Proficiency) -> Form {
    form.controls
        .retain(|m| m.command.is_some() && m.level <= level);
    form
}
