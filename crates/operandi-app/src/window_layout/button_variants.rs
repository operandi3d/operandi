//! MB-01..15 (specifications/modifier-buttons): buttons that turn into a
//! different command while Shift, Ctrl or Alt (Option) is held.
//!
//! One rule, no per-button code: a button is a base action plus a list of
//! (modifier set → action) variants, resolved against the live modifier
//! state. Most variants come from *templates* applied by the kind of the
//! base action (every component-mode button gets Alt → Convert); a button
//! can add its own variants on top or opt out of a template.
//!
//! Label, lit state, enabled state, shortcut hint and tooltip all come from
//! the resolved action, so they are correct for free. Cmd is never a
//! variant key (MB-Q6), and a held set with no exact variant shows the base
//! (MB-Q3: extra keys cancel out).
use super::*;

/// Shift, Ctrl and Alt (Option on macOS). Cmd is not a member on purpose.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct Mods(u8);

impl Mods {
    pub(super) const NONE: Self = Self(0);
    pub(super) const SHIFT: Self = Self(1);
    pub(super) const CTRL: Self = Self(2);
    pub(super) const ALT: Self = Self(4);

    /// The physical Control key counts as Ctrl on macOS too; Cmd is ignored.
    pub(super) fn from_egui(m: Modifiers) -> Self {
        let mut mods = Self::NONE;
        if m.shift {
            mods = mods.with(Self::SHIFT);
        }
        if m.ctrl {
            mods = mods.with(Self::CTRL);
        }
        if m.alt {
            mods = mods.with(Self::ALT);
        }
        mods
    }
    pub(super) const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    const fn without(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }
    const fn common(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
    /// Tooltip name, e.g. "Shift+Option" on macOS, "Shift+Alt" elsewhere.
    pub(super) fn label(self) -> String {
        let alt = if cfg!(target_os = "macos") {
            "Option"
        } else {
            "Alt"
        };
        [
            (Self::SHIFT, "Shift"),
            (Self::CTRL, "Ctrl"),
            (Self::ALT, alt),
        ]
        .into_iter()
        .filter(|(m, _)| self.common(*m) == *m)
        .map(|(_, name)| name)
        .collect::<Vec<_>>()
        .join("+")
    }
}

/// A button that carries variants. The key is also its accessibility and
/// test name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum ButtonId {
    Vertices,
    Edges,
    Polygons,
}

impl ButtonId {
    pub(super) fn base(self) -> ButtonAction {
        match self {
            Self::Vertices => ButtonAction::Mode(Mode::Vertex),
            Self::Edges => ButtonAction::Mode(Mode::Edge),
            Self::Polygons => ButtonAction::Mode(Mode::Polygon),
        }
    }
}

/// What a button (or one of its variants) does when clicked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ButtonAction {
    /// Switch component mode (clicking the active mode goes back to Items).
    Mode(Mode),
    Selection(SelectionOperation),
}

impl ButtonAction {
    /// The label shown when nothing overrides it.
    fn label(self) -> &'static str {
        match self {
            Self::Mode(Mode::Vertex) => "Vertices",
            Self::Mode(Mode::Edge) => "Edges",
            Self::Mode(Mode::Polygon) => "Polygons",
            Self::Mode(Mode::Item) => "Items",
            Self::Selection(op) => op.label(),
        }
    }
    /// Input-map action whose binding is the shortcut hint. A variant's
    /// action has its own (usually none), so the base hint disappears.
    pub(super) fn shortcut_action(self) -> Option<&'static str> {
        match self {
            Self::Mode(_) => Some(self.label()),
            Self::Selection(op) => Some(op.label()),
        }
    }
    fn tooltip_action(self) -> &'static str {
        self.shortcut_action().unwrap_or_default()
    }
    fn description(self) -> &'static str {
        match self {
            Self::Mode(_) => "Click the active component mode again for Items",
            Self::Selection(SelectionOperation::Convert(Mode::Vertex)) => {
                "convert the current selection to vertices"
            }
            Self::Selection(SelectionOperation::Convert(Mode::Edge)) => {
                "convert the current selection to edges"
            }
            Self::Selection(SelectionOperation::Convert(_)) => {
                "convert the current selection to polygons"
            }
            Self::Selection(SelectionOperation::Boundary) => {
                "replace the selection with the border edges of the selected polygons (Polygon mode)"
            }
            Self::Selection(SelectionOperation::BoundaryAdd) => {
                "add the border edges of the selected polygons to the edge selection (Polygon mode)"
            }
            Self::Selection(_) => "selection command",
        }
    }
}

/// One entry of a button's table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Variant {
    pub mods: Mods,
    pub action: ButtonAction,
    /// None keeps the base label.
    pub label: Option<&'static str>,
}

/// Patterns stated once and applied by the kind of a button's base action.
/// A template adds nothing where its target action doesn't exist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Template {
    /// Component-mode buttons: Alt → "Convert" the selection to that mode.
    /// (The Create tab's primitive-quick Shift / Ctrl variants live in its
    /// form since layouts stage 3 batch 7.)
    ConvertSelection,
}

impl Template {
    const ALL: [Self; 1] = [Self::ConvertSelection];

    fn variants(self, base: ButtonAction) -> Vec<Variant> {
        match (self, base) {
            (Self::ConvertSelection, ButtonAction::Mode(mode)) if mode != Mode::Item => {
                vec![Variant {
                    mods: Mods::ALT,
                    action: ButtonAction::Selection(SelectionOperation::Convert(mode)),
                    label: Some("Convert"),
                }]
            }
            _ => Vec::new(),
        }
    }
}

/// Per-button additions and opt-outs. Own variants replace a template's
/// variant on the same keys.
struct Overrides {
    variants: &'static [Variant],
    opt_out: &'static [Template],
}

fn overrides(button: ButtonId) -> Overrides {
    match button {
        ButtonId::Edges => Overrides {
            variants: &[
                // MB-Q9 (Jason, 2026-11-05): Bounds replaces the selection
                // with the border edges; Boundary adds them to the edges
                // already selected. Both switch to Edges.
                Variant {
                    mods: Mods::SHIFT,
                    action: ButtonAction::Selection(SelectionOperation::Boundary),
                    label: Some("Bounds"),
                },
                Variant {
                    mods: Mods::CTRL,
                    action: ButtonAction::Selection(SelectionOperation::BoundaryAdd),
                    label: Some("Boundary"),
                },
            ],
            opt_out: &[],
        },
        ButtonId::Vertices | ButtonId::Polygons => Overrides {
            variants: &[],
            opt_out: &[],
        },
    }
}

/// The button's whole variant table, sorted by keys (Shift, Ctrl, Alt, then
/// pairs), at most one variant per key set.
pub(super) fn variants(button: ButtonId) -> Vec<Variant> {
    let own = overrides(button);
    let mut table: BTreeMap<Mods, Variant> = BTreeMap::new();
    for template in Template::ALL {
        if own.opt_out.contains(&template) {
            continue;
        }
        for v in template.variants(button.base()) {
            table.insert(v.mods, v);
        }
    }
    for v in own.variants {
        table.insert(v.mods, *v);
    }
    table.into_values().collect()
}

/// What a button shows for a given held key set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Shown {
    pub action: ButtonAction,
    pub label: &'static str,
    /// The keys of the variant shown; None for the base.
    pub variant: Option<Mods>,
}

pub(super) fn resolve(button: ButtonId, held: Mods) -> Shown {
    let base = button.base();
    variants(button)
        .into_iter()
        .find(|v| held != Mods::NONE && v.mods == held)
        .map(|v| Shown {
            action: v.action,
            label: v.label.unwrap_or(base.label()),
            variant: Some(v.mods),
        })
        .unwrap_or(Shown {
            action: base,
            label: base.label(),
            variant: None,
        })
}

/// MB-Q7 (Operandi addition): one line listing the hidden variants.
pub(super) fn variant_line(button: ButtonId) -> String {
    let base = button.base().label();
    variants(button)
        .iter()
        .map(|v| format!("{}: {}", v.mods.label(), v.label.unwrap_or(base)))
        .collect::<Vec<_>>()
        .join(" · ")
}

/// App-wide modifier state for variant buttons.
#[derive(Debug, Default)]
pub(super) struct State {
    /// Keys the buttons react to this frame.
    held: Mods,
    /// Keys reported held when the window lost focus. They are ignored until
    /// released, so a key let go in another app can't leave a stuck variant
    /// (MB-09).
    stale: Mods,
    /// The variant keys captured at mouse press, until the click resolves
    /// (MB-10/12: what was shown at press is what runs).
    press: Option<(ButtonId, Mods)>,
    /// Where each variant button was drawn this frame (tests find them).
    pub rects: BTreeMap<ButtonId, Rect>,
}

impl State {
    /// Once per frame, before any button is drawn. Works whatever has
    /// keyboard focus, text fields included (MB-15).
    pub(super) fn update(&mut self, ctx: &Context) {
        let (raw, focused, lost) = ctx.input(|i| {
            (
                Mods::from_egui(i.modifiers),
                i.focused,
                i.events
                    .iter()
                    .any(|e| matches!(e, Event::WindowFocused(false))),
            )
        });
        self.stale = self.stale.common(raw);
        if lost || !focused {
            self.stale = raw;
        }
        self.held = raw.without(self.stale);
        if ctx.input(|i| !i.pointer.any_down() && !i.pointer.any_released()) {
            self.press = None;
        }
    }
    pub(super) fn held(&self) -> Mods {
        self.held
    }
    /// The variant keys of a primary press on `rect` this frame, stale keys
    /// left out (MB-09/10: what is shown at press is what runs).
    pub(super) fn press_on(&self, ui: &Ui, rect: Rect) -> Option<Mods> {
        ui.input(|i| {
            i.events.iter().find_map(|e| match e {
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers,
                } if rect.contains(*pos) => Some(Mods::from_egui(*modifiers).without(self.stale)),
                _ => None,
            })
        })
    }
}

/// Fit `text` into `width`, ending in an ellipsis when it is cut, so a
/// longer variant label never widens its button.
pub(super) fn fit_label(ui: &Ui, text: &str, font: &FontId, width: f32) -> String {
    let measure = |s: &str| {
        ui.painter()
            .layout_no_wrap(s.to_owned(), font.clone(), Color32::WHITE)
            .size()
            .x
    };
    if measure(text) <= width {
        return text.to_owned();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let cut: String = chars.iter().collect::<String>().trim_end().to_owned() + "…";
        if measure(&cut) <= width {
            return cut;
        }
    }
    "…".into()
}

impl Operandi {
    /// What `button` shows right now.
    pub(super) fn shown(&self, button: ButtonId) -> Shown {
        resolve(button, self.button_variants.held())
    }
    /// Lit while the resolved action's mode or dialog is current. A variant
    /// with no on/off state is never lit (MB-06).
    pub(super) fn action_active(&self, action: ButtonAction) -> bool {
        match action {
            // A tag selection mode is Polygon mode inside, but only its
            // own button lights (the modes are exclusive).
            ButtonAction::Mode(mode) => {
                self.editor.scene.selection.mode == mode && !self.tag_mode_active()
            }
            ButtonAction::Selection(_) => false,
        }
    }
    pub(super) fn action_enabled(&self, action: ButtonAction) -> bool {
        self.action_unavailable(action).is_none()
    }
    /// Why a button action is greyed now (`None` = it can run).
    pub(super) fn action_unavailable(&self, action: ButtonAction) -> Option<&'static str> {
        use crate::unavailable::{TOOL_ACTIVE, first, unless};
        let idle = unless(self.editor.session.live.is_none(), TOOL_ACTIVE);
        match action {
            ButtonAction::Selection(
                SelectionOperation::Boundary | SelectionOperation::BoundaryAdd,
            ) => first([
                idle,
                unless(
                    self.editor.scene.selection.mode == Mode::Polygon,
                    "Switch to Polygons mode first",
                ),
            ]),
            ButtonAction::Selection(_) => idle,
            ButtonAction::Mode(_) => None,
        }
    }
    /// Tooltip of what's shown, plus the variant list.
    pub(super) fn variant_tooltip(&self, button: ButtonId) -> String {
        let shown = self.shown(button);
        let line = format!(
            "{}{} · {}\n{}",
            shown.label,
            self.shortcut(shown.action.tooltip_action()),
            shown.action.description(),
            variant_line(button)
        );
        line.trim_end().to_owned()
    }
    /// Call with a variant button's response every frame it is drawn:
    /// captures the keys at mouse press and, on click, runs what was shown
    /// then. The modifier is consumed by the choice; the action never sees
    /// it as a second meaning.
    pub(super) fn variant_click(&mut self, ui: &Ui, button: ButtonId, response: &Response) {
        let rect = response.rect;
        self.button_variants.rects.insert(button, rect);
        if let Some(mods) = self.button_variants.press_on(ui, rect) {
            self.button_variants.press = Some((button, mods));
        }
        if !response.clicked() {
            return;
        }
        let mods = match self.button_variants.press.take() {
            Some((pressed_on, mods)) if pressed_on == button => mods,
            _ => self.button_variants.held(),
        };
        let action = resolve(button, mods).action;
        if self.action_enabled(action) {
            self.run_button_action(action);
        }
    }
    pub(super) fn run_button_action(&mut self, action: ButtonAction) {
        match action {
            ButtonAction::Mode(mode) => {
                let mode = if self.editor.scene.selection.mode == mode {
                    Mode::Item
                } else {
                    mode
                };
                self.run(Command::Mode(mode));
            }
            ButtonAction::Selection(op) => self.run(Command::Selection(op)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_follow_templates_and_overrides() {
        let edges = variants(ButtonId::Edges);
        assert_eq!(
            edges.iter().map(|v| v.mods).collect::<Vec<_>>(),
            [Mods::SHIFT, Mods::CTRL, Mods::ALT]
        );
        for button in [ButtonId::Vertices, ButtonId::Polygons] {
            let table = variants(button);
            assert_eq!(table.len(), 1);
            assert_eq!(table[0].mods, Mods::ALT);
            assert_eq!(table[0].label, Some("Convert"));
        }
    }

    #[test]
    fn mb_13_unmatched_pairs_cancel_to_the_base() {
        let pair = Mods::SHIFT.with(Mods::ALT);
        assert_eq!(resolve(ButtonId::Edges, pair).label, "Edges");
        assert_eq!(resolve(ButtonId::Edges, pair).variant, None);
        let all = pair.with(Mods::CTRL);
        assert_eq!(resolve(ButtonId::Vertices, all).label, "Vertices");
    }

    #[test]
    fn mb_05_cmd_is_never_a_variant_key() {
        let cmd = Modifiers {
            mac_cmd: true,
            command: true,
            ..Modifiers::NONE
        };
        assert_eq!(Mods::from_egui(cmd), Mods::NONE);
        // Cmd on top of Alt still shows the Alt variant.
        let alt_cmd = Modifiers { alt: true, ..cmd };
        assert_eq!(Mods::from_egui(alt_cmd), Mods::ALT);
    }

    #[test]
    fn variant_line_names_every_variant() {
        let alt = Mods::ALT.label();
        assert_eq!(
            variant_line(ButtonId::Edges),
            format!("Shift: Bounds · Ctrl: Boundary · {alt}: Convert")
        );
    }
}
