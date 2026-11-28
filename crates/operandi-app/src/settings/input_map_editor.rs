//! Input Map editor: view, search and rebind the shortcuts in the editable
//! input map (`input::ACTIONS` plus the user's saved overrides).
//!
//! The editor works on `bindings_draft`; nothing changes until Save Input
//! Map writes the draft to disk and makes it live. Shortcuts are assigned by
//! pressing them, not by typing their names. Two actions may share a key
//! only when they never apply at the same time (`input::contexts_overlap`),
//! the same rule the saved map has always followed.
//!
//! Layouts stage 5 (LAY-20, UIC-201): "Keys for" chooses the global map or
//! the current layout's own keys (`layout_keys_draft`). A layout key works
//! only in that layout; rows it sets or hides carry a marker whose tooltip
//! names the layout. Save writes both (layout_keys.rs).
use crate::layout_keys::{self, Mark};
use crate::unavailable::Gate;
use crate::{Operandi, input, theme};
use eframe::egui::{
    self, Align, Context, Event, Id, Key, Layout, Modifiers, RichText, ScrollArea, Sense, vec2,
};
use operandi_core::Mode;
use std::collections::BTreeMap;

/// What a press of keys is being recorded for.
#[derive(Clone, Debug, PartialEq)]
pub enum Recording {
    /// Assign the next chord to this action.
    Action(String),
    /// Search for the next chord ("what does this key do?").
    Search,
}

/// Shown after an assignment that clashes with other actions.
#[derive(Clone, Debug, PartialEq)]
pub struct Clash {
    pub action: String,
    pub chord: String,
    pub others: Vec<String>,
    /// The action's binding before the assignment, for Undo. `None`: the
    /// layout had no key of its own for it (it used the global one).
    pub previous: Option<String>,
}

/// Which key map the editor shows and changes (LAY-20).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Scope {
    /// The global input map: every layout uses it where it sets nothing.
    #[default]
    Global,
    /// The current layout's own keys, over the global map.
    Layout,
}

#[derive(Clone, Debug, Default)]
pub struct State {
    pub search: String,
    /// Only actions that apply in this selection mode.
    pub mode: Option<Mode>,
    pub only_assigned: bool,
    pub recording: Option<Recording>,
    pub clash: Option<Clash>,
    /// Global map or this layout's keys.
    pub scope: Scope,
    /// In the layout scope, only the actions this layout sets.
    pub only_layout: bool,
}

impl State {
    /// Forget per-session editing state when the editor closes.
    fn close(&mut self) {
        self.recording = None;
        self.clash = None;
    }
}

/// What a frame's key events mean while recording.
#[derive(Clone, Debug, PartialEq)]
pub enum Captured {
    /// A chord, in the input map's text form.
    Chord(String),
    /// Plain Escape: stop recording without changing anything.
    Cancel,
    /// A key that can't be bound (plain Space, which navigation reserves).
    Reserved,
}

/// The key's name in the input map's text form. Symbols stay symbols
/// (`[`, `,`), arrows keep the `Arrow` prefix used by the factory map.
pub fn key_text(key: Key) -> String {
    match key {
        Key::ArrowUp => "ArrowUp".into(),
        Key::ArrowDown => "ArrowDown".into(),
        Key::ArrowLeft => "ArrowLeft".into(),
        Key::ArrowRight => "ArrowRight".into(),
        _ => {
            let symbol = key.symbol_or_name();
            // "+" separates chord parts, so Plus keeps its name.
            if symbol.len() == 1 && symbol.is_ascii() && symbol != "+" {
                symbol.into()
            } else {
                key.name().into()
            }
        }
    }
}

/// A chord in text form, modifiers in a fixed order (Ctrl, Alt, Shift, Cmd).
/// `keypad` names keypad digits and the keypad decimal point.
pub fn chord_text(mods: Modifiers, key: Key, keypad: bool) -> String {
    let mut parts: Vec<String> = vec![];
    if mods.ctrl {
        parts.push("Ctrl".into());
    }
    if mods.alt {
        parts.push("Alt".into());
    }
    if mods.shift {
        parts.push("Shift".into());
    }
    if mods.mac_cmd {
        parts.push("Cmd".into());
    }
    let name = key_text(key);
    parts.push(match (keypad, key) {
        (true, Key::Period) => "NumpadDecimal".into(),
        (true, _) if name.len() == 1 && name.as_bytes()[0].is_ascii_digit() => {
            format!("Numpad{name}")
        }
        _ => name,
    });
    parts.join("+")
}

/// Turn the frame's key events into a captured chord. `numbers` tells the
/// main row's digits from the keypad's (the native layer reports which).
pub fn capture(events: &[Event], numbers: &[crate::native::NumberKey]) -> Option<Captured> {
    events.iter().find_map(|event| match event {
        Event::Key {
            key,
            pressed: true,
            repeat: false,
            modifiers,
            ..
        } => {
            if *key == Key::Escape && modifiers.is_none() {
                return Some(Captured::Cancel);
            }
            if *key == Key::Space && modifiers.is_none() {
                return Some(Captured::Reserved);
            }
            // Shift+] arrives as "}": record the key under it, as the
            // factory bindings spell it ("Shift+]").
            let key = &if modifiers.shift {
                input::unshifted(*key)
            } else {
                *key
            };
            let digit = if *key == Key::Period {
                Some(10)
            } else {
                key.name().parse::<u8>().ok().filter(|&d| d <= 9)
            };
            let keypad = digit.is_some_and(|digit| {
                numbers
                    .iter()
                    .any(|n| n.digit == digit && n.pressed && n.keypad)
            });
            Some(Captured::Chord(chord_text(*modifiers, *key, keypad)))
        }
        // The window turns Cmd+X/C/V into clipboard events, sometimes
        // instead of the key.
        Event::Cut => Some(Captured::Chord("Cmd+X".into())),
        Event::Copy => Some(Captured::Chord("Cmd+C".into())),
        Event::Paste(_) => Some(Captured::Chord("Cmd+V".into())),
        _ => None,
    })
}

/// Same chord, including the keypad/main-row distinction.
pub(crate) fn same_chord(a: &str, b: &str) -> bool {
    match (input::chord(a), input::chord(b)) {
        (Some(x), Some(y)) => {
            format!("{x:?}") == format!("{y:?}") && a.contains("Numpad") == b.contains("Numpad")
        }
        _ => false,
    }
}

/// Other actions bound to `chord` that can apply at the same time as
/// `action`.
pub fn clashes(bindings: &BTreeMap<String, String>, action: &str, chord: &str) -> Vec<String> {
    if chord.is_empty() {
        return vec![];
    }
    bindings
        .iter()
        .filter(|(other, binding)| {
            other.as_str() != action
                && same_chord(binding, chord)
                && input::contexts_overlap(action, other)
        })
        .map(|(other, _)| other.clone())
        .collect()
}

/// Other actions sharing `chord` that never apply at the same time as
/// `action` (shown as information, not a problem).
pub fn shared(bindings: &BTreeMap<String, String>, action: &str, chord: &str) -> Vec<String> {
    if chord.is_empty() {
        return vec![];
    }
    bindings
        .iter()
        .filter(|(other, binding)| {
            other.as_str() != action
                && same_chord(binding, chord)
                && !input::contexts_overlap(action, other)
        })
        .map(|(other, _)| other.clone())
        .collect()
}

/// Actions with a problem: a chord that doesn't parse, or one shared with
/// an action that can apply at the same time.
pub fn problems(bindings: &BTreeMap<String, String>) -> BTreeMap<String, Vec<String>> {
    bindings
        .iter()
        .filter(|(_, b)| !b.is_empty())
        .filter_map(|(action, binding)| {
            if input::chord(binding).is_none() || input::reserved_file_chord(action, binding) {
                return Some((action.clone(), vec![]));
            }
            let others = clashes(bindings, action, binding);
            (!others.is_empty()).then(|| (action.clone(), others))
        })
        .collect()
}

/// Problems in `layout` over `global`: keys of the layout that clash with
/// each other (global keys they share a context with are hidden, not
/// clashes).
pub fn problems_in_layout(
    global: &BTreeMap<String, String>,
    layout: &BTreeMap<String, String>,
) -> usize {
    if layout.is_empty() {
        return 0;
    }
    problems(&layout_keys::layered(global, layout))
        .keys()
        .filter(|a| layout.contains_key(*a))
        .count()
}

/// Room for the clash banner (two lines and a row of buttons).
const NOTICE_HEIGHT: f32 = 64.;

const MODES: [(Mode, &str); 4] = [
    (Mode::Vertex, "Vertices"),
    (Mode::Edge, "Edges"),
    (Mode::Polygon, "Polygons"),
    (Mode::Item, "Items"),
];

/// Where an action's shortcut works, in words.
pub fn context(action: &str) -> String {
    let modes: Vec<&str> = MODES
        .iter()
        .filter(|(mode, _)| {
            input::available(action, *mode, false) || input::available(action, *mode, true)
        })
        .map(|(_, name)| *name)
        .collect();
    let idle = MODES
        .iter()
        .any(|(mode, _)| input::available(action, *mode, false));
    let live = MODES
        .iter()
        .any(|(mode, _)| input::available(action, *mode, true));
    let mut text = if modes.len() == MODES.len() {
        "Any mode".to_owned()
    } else {
        modes.join(", ")
    };
    if !idle {
        text = if modes.len() == MODES.len() {
            "While a tool is active".into()
        } else {
            format!("{text}, while a tool is active")
        };
    } else if !live {
        text.push_str(", no active tool");
    }
    // Extra conditions the shortcut dispatcher checks.
    if crate::image_viewer_ui::is_viewer_action(action) {
        text = "Render window focused".into();
    } else if action.ends_with(" View") || crate::viewport_buttons::pointer_view_action(action) {
        text.push_str(", pointer over a viewport (acts on that one)");
    } else if crate::viewport_buttons::hovered_view_action(action) {
        text.push_str(", acts on the viewport under the pointer");
    } else if action.starts_with("Delete Item") {
        text.push_str(
            ", Items list focused (over a viewport: deletes the selected points, edges or polygons with the points left unused, or the items)",
        );
    } else if action == "New Mesh" {
        text.push_str(", pointer over the Items list");
    } else if action == "Drop Cube" {
        text = "While the Cube tool is active".into();
    }
    text
}

/// Whether a row passes the editor's search and filters.
pub fn visible(state: &State, action: &str, binding: &str) -> bool {
    if state.only_assigned && binding.is_empty() {
        return false;
    }
    if let Some(mode) = state.mode
        && !(input::available(action, mode, false) || input::available(action, mode, true))
    {
        return false;
    }
    let query = state.search.trim();
    if query.is_empty() {
        return true;
    }
    // A searched chord ("Shift+X", or one recorded with Find by Key) matches
    // that binding exactly; other text matches names.
    if input::chord(query).is_some() && (query.contains('+') || query.chars().count() == 1) {
        if same_chord(binding, query) {
            return true;
        }
        if query.contains('+') {
            return false;
        }
    }
    let query = query.to_lowercase();
    action.to_lowercase().contains(&query) || binding.to_lowercase() == query
}

fn shortcut_button(text: RichText, recording: bool, width: f32) -> egui::Button<'static> {
    let button = egui::Button::new(text).min_size(vec2(width, 0.));
    if recording {
        button.fill(theme::orange())
    } else {
        button
    }
}

impl Operandi {
    /// Called each frame while the Input Map is open (and no other modal is).
    pub(crate) fn input_map_window(&mut self, ctx: &Context) {
        let was_recording = self.input_map_ui.recording.is_some();
        if let Some(target) = self.input_map_ui.recording.clone() {
            let captured = ctx.input(|i| capture(&i.events, &self.native_numbers));
            match captured {
                Some(Captured::Cancel) => self.input_map_ui.recording = None,
                Some(Captured::Reserved) => {
                    self.status = "Plain Space is reserved for navigation; press another key or add a modifier".into();
                }
                Some(Captured::Chord(chord)) => {
                    self.input_map_ui.recording = None;
                    match target {
                        Recording::Search => {
                            self.input_map_ui.search = chord;
                            self.input_map_ui.mode = None;
                            self.input_map_ui.only_assigned = false;
                        }
                        Recording::Action(action) => self.assign(&action, chord),
                    }
                }
                None => {}
            }
            // Keep the keys away from text fields and focus travel.
            ctx.memory_mut(|m| {
                if let Some(id) = m.focused() {
                    m.surrender_focus(id);
                }
            });
        }
        let shown = self.input_map_shown();
        let problems = problems(&shown);
        // Save needs both maps clean, whichever one shows.
        let other_problems = match self.input_map_ui.scope {
            Scope::Global => problems_in_layout(&self.bindings_draft, &self.layout_keys_draft),
            Scope::Layout => self::problems(&self.bindings_draft)
                .keys()
                .filter(|a| !problems.contains_key(*a))
                .count(),
        };
        let defaults = input::default_bindings();
        let layout_name = self.layouts.current.clone();
        let mut close = false;
        crate::dialog_place::modal(ctx, Id::new("Input Map")).show(ctx, |ui| {
            ui.set_width(700.);
            ui.heading("Input Map");
            self.input_map_filters(ui);
            ui.add_space(4.);
            // A fixed-height slot for the clash banner or the hint, so the
            // window doesn't resize (and move its buttons) when one appears.
            let width = ui.available_width();
            ui.allocate_ui(vec2(width, NOTICE_HEIGHT), |ui| {
                ui.set_min_size(vec2(width, NOTICE_HEIGHT));
                if !self.input_map_clash(ui) {
                    let hint = match self.input_map_ui.scope {
                        Scope::Global => "Click a shortcut, then press the keys you want (Esc cancels). \
                             Actions that never apply at the same time can share a key, \
                             like B for Bevel in Polygons and Edge Bevel in Edges."
                            .to_owned(),
                        Scope::Layout => format!(
                            "Keys you set here work only in the {layout_name} layout; every other \
                             key comes from the global map. ◆ marks this layout's keys, ◇ a \
                             global key one of them hides here."
                        ),
                    };
                    ui.label(RichText::new(hint).color(theme::weak_text()));
                }
            });
            let state = &self.input_map_ui;
            let only_layout = state.scope == Scope::Layout && state.only_layout;
            // UIC-311: every pie can be bound, a user's own too; its row is
            // the line that opens it, named "Pie: <name>".
            let all = self.input_map_actions();
            let rows: Vec<String> = all
                .iter()
                .filter(|name| {
                    let binding = shown.get(*name).map_or("", String::as_str);
                    (visible(state, name, binding)
                        || (crate::pie_menu::is_pie(name)
                            && visible(state, &self.action_label(name), binding)))
                        && (!only_layout || self.layout_keys_draft.contains_key(*name))
                })
                .cloned()
                .collect();
            let assigned = shown.values().filter(|b| !b.is_empty()).count();
            ui.label(
                RichText::new(format!(
                    "Showing {} of {} actions · {assigned} with shortcuts",
                    rows.len(),
                    all.len()
                ))
                .small()
                .color(theme::weak_text()),
            );
            ui.separator();
            ScrollArea::vertical()
                // A fixed size keeps the window (and its buttons) still
                // while filters change the row count.
                .min_scrolled_height(420.)
                .max_height(420.)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Grid::new("input-map-rows")
                        .num_columns(4)
                        .striped(true)
                        .spacing(vec2(12., 4.))
                        .show(ui, |ui| {
                            ui.label(RichText::new("Action").strong());
                            ui.label(RichText::new("Works in").strong());
                            ui.label(RichText::new("Shortcut").strong());
                            ui.label("");
                            ui.end_row();
                            for name in &rows {
                                self.input_map_row(ui, name, &shown, &defaults, &problems);
                                ui.end_row();
                            }
                        });
                    if rows.is_empty() {
                        ui.label(
                            RichText::new("No actions match.").color(theme::weak_text()),
                        );
                    }
                });
            ui.separator();
            egui::CollapsingHeader::new("Advanced")
                .id_salt("input-map-advanced")
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Cube segment detent (logical px, this session)");
                        ui.add(
                            crate::numeric_tab::DragValue::new(&mut self.segment_detent_pixels)
                                .range(1. ..=256.)
                                .speed(1.),
                        );
                    });
                });
            ui.horizontal(|ui| {
                if self.input_map_ui.scope == Scope::Global {
                    let changed = self.bindings_draft != defaults;
                    if ui
                        .gated(crate::unavailable::unless(changed, "Every shortcut is already the default"), egui::Button::new("Reset All to Defaults"))
                        .on_hover_text("Puts every global shortcut back to the factory map. Layouts' own keys stay. Nothing is saved until Save Input Map.")
                        .clicked()
                    {
                        self.bindings_draft = defaults.clone();
                        self.input_map_ui.clash = None;
                        self.status = "All shortcuts reset to defaults (not saved yet)".into();
                    }
                }
                if ui
                    .gated(
                        crate::unavailable::unless(
                            !self.layout_keys_draft.is_empty(),
                            &format!("The {layout_name} layout has no keys of its own"),
                        ),
                        egui::Button::new("Clear this layout's keys"),
                    )
                    .on_hover_text(format!(
                        "The {layout_name} layout goes back to the global keys. Nothing is saved until Save Input Map."
                    ))
                    .clicked()
                {
                    self.layout_keys_draft.clear();
                    if self.input_map_ui.scope == Scope::Layout {
                        self.input_map_ui.clash = None;
                    }
                    self.status = format!("{layout_name} uses the global keys (not saved yet)");
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let other = match self.input_map_ui.scope {
                        Scope::Global => format!("Fix the shortcuts marked in red under This layout ({layout_name}) first."),
                        Scope::Layout => "Fix the shortcuts marked in red under Global first.".to_owned(),
                    };
                    let save = ui.gated(
                        if problems.is_empty() {
                            crate::unavailable::unless(other_problems == 0, &other)
                        } else {
                            crate::unavailable::unless(false, "Fix the shortcuts marked in red first.")
                        },
                        egui::Button::new("Save Input Map"),
                    );
                    if save.clicked() {
                        self.save_input_map();
                        if !self.show_bindings {
                            close = true;
                        }
                    }
                    if ui.button("Cancel").clicked()
                        || (!was_recording && ui.input(|i| i.key_pressed(Key::Escape)))
                    {
                        self.show_bindings = false;
                        close = true;
                    }
                    let count = problems.len() + other_problems;
                    if count > 0 {
                        ui.colored_label(
                            theme::error(),
                            format!(
                                "{} shortcut{} to fix",
                                count,
                                if count == 1 { "" } else { "s" }
                            ),
                        );
                    }
                });
            });
            ui.label(&self.status);
        });
        if close {
            self.input_map_ui.close();
        }
    }

    fn input_map_filters(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // LAY-20 / UIC-201: which key map the rows show and change.
            ui.label("Keys for");
            let layout = format!("This layout ({})", self.layouts.current);
            let before = self.input_map_ui.scope;
            egui::ComboBox::from_id_salt("input-map-scope")
                .selected_text(match before {
                    Scope::Global => "Global".to_owned(),
                    Scope::Layout => layout.clone(),
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.input_map_ui.scope, Scope::Global, "Global")
                        .on_hover_text("The keys every layout uses unless it sets its own");
                    ui.selectable_value(&mut self.input_map_ui.scope, Scope::Layout, layout)
                        .on_hover_text("Keys only this layout uses; the rest come from Global");
                });
            if self.input_map_ui.scope != before {
                self.input_map_ui.recording = None;
                self.input_map_ui.clash = None;
            }
            if self.input_map_ui.scope == Scope::Layout {
                ui.checkbox(
                    &mut self.input_map_ui.only_layout,
                    "Only this layout's keys",
                );
            }
        });
        ui.horizontal(|ui| {
            let search = ui.add(
                egui::TextEdit::singleline(&mut self.input_map_ui.search)
                    .hint_text("Search actions or keys (e.g. Shift+X)")
                    .desired_width(260.),
            );
            self.text_editors.insert(search.id);
            crate::numeric_tab::register(ui, &search, Some(&self.input_map_ui.search));
            let finding = self.input_map_ui.recording == Some(Recording::Search);
            if ui
                .add(shortcut_button(
                    RichText::new(if finding {
                        "Press keys…"
                    } else {
                        "Find by Key"
                    }),
                    finding,
                    0.,
                ))
                .on_hover_text("Press a shortcut to see which actions use it")
                .clicked()
            {
                self.input_map_ui.recording = (!finding).then_some(Recording::Search);
            }
            if !self.input_map_ui.search.is_empty()
                && ui.small_button("✕").on_hover_text("Clear search").clicked()
            {
                self.input_map_ui.search.clear();
            }
            let label = MODES
                .iter()
                .find(|(m, _)| Some(*m) == self.input_map_ui.mode)
                .map_or("All modes", |(_, name)| name);
            egui::ComboBox::from_id_salt("input-map-mode")
                .selected_text(label)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.input_map_ui.mode, None, "All modes");
                    for (mode, name) in MODES {
                        ui.selectable_value(&mut self.input_map_ui.mode, Some(mode), name);
                    }
                });
            ui.checkbox(&mut self.input_map_ui.only_assigned, "Only with shortcuts");
        });
    }

    /// The clash banner; false when there's nothing to show.
    fn input_map_clash(&mut self, ui: &mut egui::Ui) -> bool {
        let Some(clash) = self.input_map_ui.clash.clone() else {
            return false;
        };
        // The banner goes away once the clash is resolved some other way.
        let shown = self.input_map_shown();
        if shown.get(&clash.action) != Some(&clash.chord)
            || clashes(&shown, &clash.action, &clash.chord).is_empty()
        {
            self.input_map_ui.clash = None;
            return false;
        }
        // Rows' names, not their lines ("Pie: View Direction").
        let others: Vec<String> = clash.others.iter().map(|o| self.action_label(o)).collect();
        let action = self.action_label(&clash.action);
        egui::Frame::group(ui.style())
            .stroke(egui::Stroke::new(1.0_f32, theme::error()))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(format!(
                    "{} is already used by {}, which can apply at the same time as {}.",
                    input::display_chord(&clash.chord),
                    others.join(", "),
                    action
                ));
                ui.horizontal(|ui| {
                    let label = if others.len() == 1 {
                        format!("Remove it from {}", others[0])
                    } else {
                        "Remove it from the others".into()
                    };
                    if ui.button(label).clicked() {
                        for other in &clash.others {
                            self.input_map_draft_mut()
                                .insert(other.clone(), String::new());
                        }
                        self.input_map_ui.clash = None;
                        self.status =
                            format!("{} now runs {}", input::display_chord(&clash.chord), action);
                    }
                    if ui
                        .button(format!("Undo (keep {} as it was)", action))
                        .clicked()
                    {
                        let draft = self.input_map_draft_mut();
                        match &clash.previous {
                            Some(previous) => {
                                draft.insert(clash.action.clone(), previous.clone());
                            }
                            None => {
                                draft.remove(&clash.action);
                            }
                        }
                        self.input_map_ui.clash = None;
                        self.status.clear();
                    }
                });
            });
        true
    }

    fn input_map_row(
        &mut self,
        ui: &mut egui::Ui,
        name: &str,
        shown: &BTreeMap<String, String>,
        defaults: &BTreeMap<String, String>,
        problems: &BTreeMap<String, Vec<String>>,
    ) {
        let scope = self.input_map_ui.scope;
        let binding = shown.get(name).cloned().unwrap_or_default();
        // In the layout scope "the default" is the global key.
        let default = match scope {
            Scope::Global => defaults.get(name).cloned().unwrap_or_default(),
            Scope::Layout => self.bindings_draft.get(name).cloned().unwrap_or_default(),
        };
        let modified = match scope {
            Scope::Global => binding != default,
            Scope::Layout => self.layout_keys_draft.contains_key(name),
        };
        let mut label = RichText::new(self.action_label(name));
        if modified {
            label = label.color(theme::orange());
        }
        let layout = self.layouts.current.clone();
        let mark = layout_keys::mark(&self.bindings_draft, &self.layout_keys_draft, name);
        ui.horizontal(|ui| {
            ui.label(label).on_hover_text(match (scope, modified) {
                (Scope::Global, true) => format!(
                    "Changed. Default: {}",
                    if default.is_empty() { "none" } else { &default }
                ),
                (Scope::Global, false) => "Default shortcut".into(),
                (Scope::Layout, true) => format!("Set for the {layout} layout"),
                (Scope::Layout, false) => "Uses the global key".into(),
            });
            // UIC-201: a key the current layout sets or hides is marked,
            // and the tooltip names the layout.
            match &mark {
                Some(Mark::Set { global }) => {
                    let mine = self.layout_keys_draft.get(name).cloned().unwrap_or_default();
                    ui.label(RichText::new("◆").color(theme::cyan()))
                        .on_hover_text(format!(
                            "The {layout} layout sets this key: {} there (global key: {})",
                            if mine.is_empty() { "none".to_owned() } else { input::display_chord(&mine) },
                            if global.is_empty() { "none".to_owned() } else { input::display_chord(global) },
                        ));
                }
                Some(Mark::Hidden { by }) => {
                    ui.label(RichText::new("◇").color(theme::cyan()))
                        .on_hover_text(format!(
                            "In the {layout} layout this key runs {by} instead (the layout's own key)"
                        ));
                }
                None => {}
            }
        });
        ui.label(
            RichText::new(context(name))
                .small()
                .color(theme::weak_text()),
        );
        let recording =
            self.input_map_ui.recording.as_ref() == Some(&Recording::Action(name.to_owned()));
        let problem = problems.get(name);
        let text = if recording {
            RichText::new("Press keys…").color(theme::on_orange())
        } else if binding.is_empty() {
            RichText::new("—").color(theme::weak_text())
        } else if problem.is_some() {
            RichText::new(input::display_chord(&binding)).color(theme::error())
        } else {
            RichText::new(input::display_chord(&binding))
        };
        let mut button = ui.add(shortcut_button(text, recording, 130.));
        button = match problem {
            Some(others) if others.is_empty() => {
                button.on_hover_text("Not a key Operandi understands; press a new one")
            }
            Some(others) => button.on_hover_text(format!(
                "Also used by {} at the same time",
                others.join(", ")
            )),
            None if recording => button.on_hover_text("Press the keys; Esc cancels"),
            None => {
                let shared = shared(shown, name, &binding);
                if shared.is_empty() {
                    button.on_hover_text("Click, then press the new shortcut")
                } else {
                    button.on_hover_text(format!(
                        "Click, then press the new shortcut.\nAlso used (in other contexts) by {}",
                        shared.join(", ")
                    ))
                }
            }
        };
        if button.clicked() {
            self.input_map_ui.recording = (!recording).then(|| Recording::Action(name.to_owned()));
        }
        ui.horizontal(|ui| {
            if ui
                .gated(
                    crate::unavailable::unless(!binding.is_empty(), "No shortcut to clear"),
                    egui::Button::new("Clear").small(),
                )
                .on_hover_text(match scope {
                    Scope::Global => "Remove this shortcut".to_owned(),
                    Scope::Layout => format!("No key for this action in the {layout} layout"),
                })
                .clicked()
            {
                self.input_map_draft_mut()
                    .insert(name.into(), String::new());
            }
            match scope {
                Scope::Global => {
                    if ui
                        .gated(
                            crate::unavailable::unless(modified, "Already the default shortcut"),
                            egui::Button::new("Reset").small(),
                        )
                        .on_hover_text(format!(
                            "Back to the default: {}",
                            if default.is_empty() { "none" } else { &default }
                        ))
                        .clicked()
                    {
                        self.assign(name, default.clone());
                    }
                }
                Scope::Layout => {
                    if ui
                        .gated(
                            crate::unavailable::unless(modified, "Already uses the global key"),
                            egui::Button::new("Use Global").small(),
                        )
                        .on_hover_text(format!(
                            "Back to the global key: {}",
                            if default.is_empty() { "none" } else { &default }
                        ))
                        .clicked()
                    {
                        self.layout_keys_draft.remove(name);
                        self.input_map_ui.clash = None;
                    }
                }
            }
            // Keep row heights steady when buttons are disabled.
            ui.allocate_exact_size(vec2(0., 0.), Sense::hover());
        });
    }

    /// Every row the Input Map lists: the factory actions, then any pie
    /// they don't already open (a user's pies, UIC-311).
    pub(crate) fn input_map_actions(&self) -> Vec<String> {
        let mut rows: Vec<String> = input::ACTIONS
            .iter()
            .map(|&(name, _)| name.to_owned())
            .collect();
        for id in self.pie_ids() {
            let line = crate::pie_menu::line(&id);
            if !rows.contains(&line) {
                rows.push(line);
            }
        }
        rows
    }

    /// Open the Input Map on the saved maps: the global one and the
    /// current layout's keys.
    pub(crate) fn open_input_map(&mut self) {
        self.bindings_draft = self.global_bindings.clone();
        self.layout_keys_draft = self.current_layout_keys();
        self.show_bindings = true;
    }

    /// The map the rows show: the global draft, or the layout's keys over
    /// it.
    pub(crate) fn input_map_shown(&self) -> BTreeMap<String, String> {
        match self.input_map_ui.scope {
            Scope::Global => self.bindings_draft.clone(),
            Scope::Layout => layout_keys::layered(&self.bindings_draft, &self.layout_keys_draft),
        }
    }

    /// The draft the editor changes in its scope.
    fn input_map_draft_mut(&mut self) -> &mut BTreeMap<String, String> {
        match self.input_map_ui.scope {
            Scope::Global => &mut self.bindings_draft,
            Scope::Layout => &mut self.layout_keys_draft,
        }
    }

    /// Put `chord` on `action` in the draft and report any clash.
    pub(crate) fn assign(&mut self, action: &str, chord: String) {
        if input::reserved_file_chord(action, &chord) {
            self.status = format!(
                "{} is reserved for the File menu",
                input::display_chord(&chord)
            );
            return;
        }
        let scope = self.input_map_ui.scope;
        let before = self.input_map_shown();
        let draft = self.input_map_draft_mut();
        let mut previous = draft.insert(action.to_owned(), chord.clone());
        if scope == Scope::Global {
            previous = Some(previous.unwrap_or_default());
        }
        let shown = self.input_map_shown();
        let others = clashes(&shown, action, &chord);
        self.input_map_ui.clash = (!others.is_empty()).then(|| Clash {
            action: action.to_owned(),
            chord: chord.clone(),
            others,
            previous,
        });
        let name = self.action_label(action);
        let layout = &self.layouts.current;
        self.status = match scope {
            Scope::Global if chord.is_empty() => format!("{name} has no shortcut"),
            Scope::Global => format!("{name}: {chord} (not saved yet)"),
            Scope::Layout if chord.is_empty() => {
                format!("{name} has no key in the {layout} layout (not saved yet)")
            }
            Scope::Layout => {
                // Global keys this one now hides in the layout.
                let hidden: Vec<String> = before
                    .iter()
                    .filter(|(other, b)| {
                        other.as_str() != action
                            && !b.is_empty()
                            && shown.get(*other).is_some_and(String::is_empty)
                    })
                    .map(|(other, _)| other.clone())
                    .collect();
                if hidden.is_empty() {
                    format!("{name}: {chord} in the {layout} layout (not saved yet)")
                } else {
                    format!(
                        "{name}: {chord} in the {layout} layout, instead of {} there (not saved yet)",
                        hidden.join(", ")
                    )
                }
            }
        };
    }

    /// Write the drafts (the input map file and the current layout's keys)
    /// and make them live.
    pub(crate) fn save_input_map(&mut self) {
        // Chords must be unique unless their actions never share a context,
        // globally and with this layout's keys on top.
        if !input::bindings_valid(&self.bindings_draft)
            || problems_in_layout(&self.bindings_draft, &self.layout_keys_draft) > 0
        {
            self.status = "Use valid unique chords; plain Space is reserved; only supported keypad digit/decimal chords may be assigned".into();
            return;
        }
        let path = self.environment.paths().input_map();
        let temp = path.with_extension("json.tmp");
        let result = serde_json::to_vec_pretty(&self.bindings_draft)
            .map_err(|e| e.to_string())
            .and_then(|b| {
                std::fs::write(&temp, b)
                    .and_then(|()| std::fs::rename(&temp, &path))
                    .map_err(|e| e.to_string())
            });
        match result {
            Ok(()) => {
                self.global_bindings = self.bindings_draft.clone();
                let layout = self.layouts.current.clone();
                if let Err(e) = self.set_layout_keys(&layout, self.layout_keys_draft.clone()) {
                    self.status = e;
                    return;
                }
                self.refresh_bindings();
                self.status = "Input map saved".into();
                self.show_bindings = false;
            }
            Err(e) => self.status = e,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_round_trips_through_its_text() {
        for &key in Key::ALL {
            for mods in [
                Modifiers::NONE,
                Modifiers::SHIFT,
                Modifiers::ALT | Modifiers::SHIFT,
                Modifiers::CTRL,
                crate::tests::PLATFORM_CMD | Modifiers::SHIFT,
            ] {
                if key == Key::Space && mods.is_none() {
                    continue;
                }
                let text = chord_text(mods, key, false);
                assert!(
                    input::matches_chord(&text, key, mods),
                    "{key:?} {mods:?} -> {text:?} doesn't parse back"
                );
            }
        }
    }

    #[test]
    fn recorded_text_matches_the_factory_spelling() {
        let cmd = Modifiers::MAC_CMD | Modifiers::COMMAND;
        assert_eq!(
            chord_text(cmd | Modifiers::SHIFT, Key::Z, false),
            "Shift+Cmd+Z"
        );
        assert_eq!(
            chord_text(Modifiers::ALT | Modifiers::SHIFT, Key::A, false),
            "Alt+Shift+A"
        );
        assert_eq!(
            chord_text(Modifiers::SHIFT, Key::ArrowUp, false),
            "Shift+ArrowUp"
        );
        assert_eq!(
            chord_text(Modifiers::SHIFT, Key::CloseBracket, false),
            "Shift+]"
        );
        assert_eq!(chord_text(cmd, Key::Comma, false), "Cmd+,");
        assert_eq!(chord_text(Modifiers::NONE, Key::Num1, true), "Numpad1");
        assert_eq!(
            chord_text(Modifiers::NONE, Key::Period, true),
            "NumpadDecimal"
        );
        assert_eq!(chord_text(Modifiers::NONE, Key::Num1, false), "1");
    }

    fn key(key: Key, modifiers: Modifiers) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    #[test]
    fn capture_handles_escape_space_keypad_and_clipboard() {
        assert_eq!(
            capture(&[key(Key::Escape, Modifiers::NONE)], &[]),
            Some(Captured::Cancel)
        );
        assert_eq!(
            capture(&[key(Key::Escape, Modifiers::SHIFT)], &[]),
            Some(Captured::Chord("Shift+Escape".into()))
        );
        assert_eq!(
            capture(&[key(Key::Space, Modifiers::NONE)], &[]),
            Some(Captured::Reserved)
        );
        let pad = crate::native::NumberKey {
            digit: 4,
            keypad: true,
            pressed: true,
        };
        assert_eq!(
            capture(&[key(Key::Num4, Modifiers::NONE)], &[pad]),
            Some(Captured::Chord("Numpad4".into()))
        );
        assert_eq!(
            capture(&[key(Key::Num4, Modifiers::NONE)], &[]),
            Some(Captured::Chord("4".into()))
        );
        assert_eq!(
            capture(&[Event::Copy], &[]),
            Some(Captured::Chord("Cmd+C".into()))
        );
        assert_eq!(capture(&[], &[]), None);
    }

    #[test]
    fn factory_map_has_no_problems_and_context_sharing_is_not_a_clash() {
        let defaults = input::default_bindings();
        assert!(problems(&defaults).is_empty());
        // B is Bevel in Polygon mode and Edge Bevel in Edge mode.
        assert!(clashes(&defaults, "Bevel", "B").is_empty());
        assert!(shared(&defaults, "Bevel", "B").contains(&"Edge Bevel".to_owned()));
    }

    #[test]
    fn a_clash_is_found_both_ways_and_numpad_differs_from_digits() {
        let mut map = input::default_bindings();
        map.insert("Move".into(), "R".into());
        assert_eq!(clashes(&map, "Move", "R"), vec!["Scale".to_owned()]);
        let problems = problems(&map);
        assert!(problems.contains_key("Move") && problems.contains_key("Scale"));
        // Numpad1 (Top View) and 1 (Vertices) are different keys.
        assert!(clashes(&map, "Top View", "Numpad1").is_empty());
        map.insert("Top View".into(), "1".into());
        assert_eq!(clashes(&map, "Top View", "1"), vec!["Vertices".to_owned()]);
    }

    #[test]
    fn contexts_read_as_words() {
        assert_eq!(context("Undo"), "Any mode");
        assert_eq!(context("Bevel"), "Polygons");
        assert_eq!(context("Loop"), "Vertices, Edges, Polygons, no active tool");
        assert_eq!(context("Drop"), "While a tool is active");
        assert_eq!(
            context("Top View"),
            "Any mode, pointer over a viewport (acts on that one)"
        );
        assert!(context("Frame All").ends_with(", acts on the viewport under the pointer"));
    }

    #[test]
    fn search_matches_names_and_keys() {
        let mut state = State::default();
        let check = |state: &State, action, binding| visible(state, action, binding);
        state.search = "bev".into();
        assert!(check(&state, "Edge Bevel", "B"));
        assert!(!check(&state, "Move", "W"));
        state.search = "Shift+X".into();
        assert!(check(&state, "Extrude", "Shift+X"));
        assert!(!check(&state, "Snapping Toggle", "X"));
        assert!(!check(&state, "Shift Thing", ""));
        // A single key finds its binding and names containing it.
        state.search = "w".into();
        assert!(check(&state, "Move", "W"));
        assert!(check(&state, "Wireframe View", "Numpad7"));
        state.search.clear();
        state.mode = Some(Mode::Edge);
        assert!(check(&state, "Edge Bevel", "B"));
        assert!(!check(&state, "Bevel", "B"));
        state.mode = None;
        state.only_assigned = true;
        assert!(!check(&state, "Cube", ""));
    }
}

#[cfg(test)]
mod window_tests {
    use super::*;
    use crate::tests::{draw, key, pointer};
    use egui::accesskit::{self, Role};

    fn frame(a: &mut Operandi, ctx: &Context, events: Vec<Event>) -> Vec<accesskit::Node> {
        draw(a, ctx, vec2(1440., 1000.), events)
            .platform_output
            .accesskit_update
            .expect("AccessKit enabled")
            .nodes
            .into_iter()
            .map(|(_, n)| n)
            .collect()
    }

    fn click(a: &mut Operandi, ctx: &Context, label: &str) {
        let nodes = frame(a, ctx, vec![]);
        // The first enabled button with this label.
        let node = nodes
            .iter()
            .find(|n| n.role() == Role::Button && n.label() == Some(label) && !n.is_disabled())
            .unwrap_or_else(|| panic!("{label} button"));
        let b = node.bounds().expect("bounds");
        let at = egui::pos2(((b.x0 + b.x1) / 2.) as f32, ((b.y0 + b.y1) / 2.) as f32);
        frame(a, ctx, vec![Event::PointerMoved(at)]);
        frame(a, ctx, vec![pointer(at, true, Modifiers::NONE)]);
        frame(a, ctx, vec![pointer(at, false, Modifiers::NONE)]);
    }

    #[test]
    fn press_to_assign_resolve_a_clash_and_save() {
        let ctx = Context::default();
        theme::install(&ctx);
        ctx.enable_accesskit();
        let mut a = Operandi::empty();
        a.show_bindings = true;
        a.bindings_draft = a.bindings.clone();
        a.input_map_ui.search = "Move".into();
        for _ in 0..2 {
            frame(&mut a, &ctx, vec![]);
        }
        click(&mut a, &ctx, "W");
        assert_eq!(
            a.input_map_ui.recording,
            Some(Recording::Action("Move".into()))
        );
        // Esc while recording cancels the recording, not the window.
        frame(&mut a, &ctx, vec![key(Key::Escape)]);
        assert_eq!(a.input_map_ui.recording, None);
        assert!(a.show_bindings);
        assert_eq!(a.bindings_draft["Move"], "W");

        click(&mut a, &ctx, "W");
        frame(&mut a, &ctx, vec![key(Key::R)]);
        assert_eq!(a.bindings_draft["Move"], "R");
        assert_eq!(
            a.input_map_ui.clash.as_ref().map(|c| c.others.clone()),
            Some(vec!["Scale".to_owned()])
        );
        assert!(!problems(&a.bindings_draft).is_empty());
        click(&mut a, &ctx, "Remove it from Scale");
        assert_eq!(a.bindings_draft["Scale"], "");
        assert!(a.input_map_ui.clash.is_none());
        // Live bindings change only on save.
        assert_eq!(a.bindings["Move"], "W");
        click(&mut a, &ctx, "Save Input Map");
        assert_eq!(a.status, "Input map saved");
        assert!(!a.show_bindings);
        assert_eq!(a.bindings["Move"], "R");
        let loaded = input::load_bindings(&a.environment.paths().input_map());
        assert_eq!(loaded["Move"], "R");
        assert_eq!(loaded["Scale"], "");
    }

    #[test]
    fn undo_on_a_clash_restores_the_old_shortcut_and_cancel_discards() {
        let ctx = Context::default();
        theme::install(&ctx);
        ctx.enable_accesskit();
        let mut a = Operandi::empty();
        a.show_bindings = true;
        // "Rotate" also finds Axis Rotate (TC-30); unbound, its Clear is
        // greyed and the click below lands on Rotate's.
        a.bindings.insert("Axis Rotate".into(), String::new());
        a.bindings_draft = a.bindings.clone();
        a.input_map_ui.search = "Rotate".into();
        frame(&mut a, &ctx, vec![]);
        click(&mut a, &ctx, "E");
        frame(&mut a, &ctx, vec![key(Key::R)]);
        click(&mut a, &ctx, "Undo (keep Rotate as it was)");
        assert_eq!(a.bindings_draft["Rotate"], "E");
        assert!(a.input_map_ui.clash.is_none());
        click(&mut a, &ctx, "Clear");
        assert_eq!(a.bindings_draft["Rotate"], "");
        click(&mut a, &ctx, "Cancel");
        assert!(!a.show_bindings);
        assert_eq!(a.bindings["Rotate"], "E");
    }
}
