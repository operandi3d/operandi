//! Animation key routing: the one place that decides whether a key is an
//! animation key here (SC-241..256, TK-63/65, TL-14/50, UIC-201).
//!
//! Context-sensitive key maps (keys per view or panel) are being
//! researched; when they land, only this module changes. Today:
//! - the Input Map actions below are Items-mode keys (component modes
//!   keep S = Slide, Shift+W and so on); `/`, `,`, `.`, Shift+R and Shift+Y
//!   come from the Animate layout's own keys (FLAGS: UIC-201);
//! - Shift/Ctrl+arrows belong to animation, not the item arrow walk
//!   (FLAGS: CP-A04);
//! - over the timeline strip a few keys act on time and keys
//!   ([`TIMELINE_KEYS`]).
use eframe::egui::{Key, Modifiers};
use operandi_core::Mode;

/// Input Map actions (shortcuts spec §14): name, the line it runs, its
/// default chord (also in `input::ACTIONS`, the input map's own list).
pub(crate) const ACTIONS: &[(&str, &str, &str)] = &[
    ("Play/Stop", "operandi.time.play", ""),
    ("Play Backwards", "operandi.time.play backwards:true", ""),
    (
        "Back One Frame",
        "operandi.time.step frames:-1",
        "Shift+ArrowLeft",
    ),
    (
        "Forward One Frame",
        "operandi.time.step frames:1",
        "Shift+ArrowRight",
    ),
    ("Step Back", "operandi.time.step frames:-1", ""),
    ("Step Forward", "operandi.time.step frames:1", ""),
    ("First Frame", "operandi.time.go to:first", "Shift+ArrowUp"),
    ("Last Frame", "operandi.time.go to:last", "Shift+ArrowDown"),
    (
        "Previous Key",
        "operandi.time.go to:prevKey",
        "Ctrl+ArrowLeft",
    ),
    ("Next Key", "operandi.time.go to:nextKey", "Ctrl+ArrowRight"),
    ("First Key", "operandi.time.go to:firstKey", "Ctrl+ArrowUp"),
    ("Last Key", "operandi.time.go to:lastKey", "Ctrl+ArrowDown"),
    ("Key Selected Channels", "operandi.key.set", "S"),
    ("Key Selected Items", "operandi.key.items", "Shift+S"),
    (
        "Key Position",
        "operandi.key.transforms type:position",
        "Shift+W",
    ),
    (
        "Key Rotation",
        "operandi.key.transforms type:rotation",
        "Shift+E",
    ),
    ("Key Scale", "operandi.key.transforms type:scale", ""),
    ("Key Transforms", "operandi.key.transforms type:all", ""),
];

/// `action` is an animation key.
pub(crate) fn is_animation_action(action: &str) -> bool {
    ACTIONS.iter().any(|(name, _, _)| *name == action)
}

/// Whether animation action `action` applies in `mode` (TK-65): Items
/// mode, with or without a tool live.
pub(crate) fn available(_action: &str, mode: Mode, _live: bool) -> bool {
    crate::anim_switch::enabled() && mode == Mode::Item
}

/// The line an animation action runs.
pub(crate) fn action_line(name: &str) -> Option<&'static str> {
    ACTIONS
        .iter()
        .find(|(n, _, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, line, _)| *line)
}

/// An arrow key with these modifiers is an animation key, so the item
/// arrow walk (CP-A04) leaves it alone.
pub(crate) fn owns_arrow(modifiers: Modifiers) -> bool {
    crate::anim_switch::enabled() && !modifiers.is_none()
}

/// Keys over the timeline strip (TL-14 fit, TL-50 marker, TL-45 delete,
/// SC-255 copy/cut/paste): modifiers, key, line. More specific chords come
/// first (egui matches Shift loosely).
pub(crate) const TIMELINE_KEYS: &[(Modifiers, Key, &str)] = &[
    (Modifiers::SHIFT, Key::A, "operandi.time.fit all:true"),
    (Modifiers::NONE, Key::A, "operandi.time.fit"),
    (Modifiers::NONE, Key::M, "operandi.marker.add"),
    (Modifiers::NONE, Key::Delete, "operandi.key.delete"),
    (Modifiers::NONE, Key::Backspace, "operandi.key.delete"),
    (Modifiers::COMMAND, Key::C, "operandi.key.copy"),
    (Modifiers::COMMAND, Key::X, "operandi.key.cut"),
    (
        Modifiers::COMMAND.plus(Modifiers::SHIFT),
        Key::V,
        "operandi.key.paste mode:insert",
    ),
    (
        Modifiers::COMMAND.plus(Modifiers::ALT),
        Key::V,
        "operandi.key.paste mode:replace",
    ),
    (Modifiers::COMMAND, Key::V, "operandi.key.paste"),
];

/// Keys over the graph editor (GE-21, SC-253/254, GE-70/72): fits,
/// Normalise, delete, copy/cut/paste. The fits are the graph's own view
/// lines; the rest are commands. Only while the pointer is over it.
pub(crate) const GRAPH_KEYS: &[(Modifiers, Key, &str)] = &[
    (Modifiers::SHIFT, Key::A, "operandi.graph.view fit:selected"),
    (Modifiers::NONE, Key::A, "operandi.graph.view fit:all"),
    (Modifiers::SHIFT, Key::F, "operandi.graph.view fit:values"),
    (
        Modifiers::COMMAND,
        Key::F,
        "operandi.graph.view fit:visible",
    ),
    (Modifiers::NONE, Key::F, "operandi.graph.view fit:time"),
    (
        Modifiers::NONE,
        Key::N,
        "operandi.graph.view normalise:toggle",
    ),
    (Modifiers::NONE, Key::Delete, "operandi.key.delete"),
    (Modifiers::NONE, Key::Backspace, "operandi.key.delete"),
    (Modifiers::COMMAND, Key::C, "operandi.key.copy"),
    (Modifiers::COMMAND, Key::X, "operandi.key.cut"),
    (
        Modifiers::COMMAND.plus(Modifiers::SHIFT),
        Key::V,
        "operandi.key.paste mode:insert",
    ),
    (
        Modifiers::COMMAND.plus(Modifiers::ALT),
        Key::V,
        "operandi.key.paste mode:replace",
    ),
    (Modifiers::COMMAND, Key::V, "operandi.key.paste"),
];

/// Takes this frame's keys from `table` (one line per key; more specific
/// chords first).
pub(crate) fn take_keys(
    input: &mut eframe::egui::InputState,
    table: &[(Modifiers, Key, &'static str)],
) -> Vec<&'static str> {
    let mut out = Vec::new();
    let mut taken: Vec<Key> = Vec::new();
    for (mods, key, line) in table {
        if taken.contains(key) {
            continue;
        }
        if input.consume_key(*mods, *key) {
            taken.push(*key);
            out.push(*line);
        }
    }
    out
}

/// Takes this frame's timeline keys (pointer over the strip, no text
/// field focused) and gives the lines they run.
pub(crate) fn take_timeline_keys(input: &mut eframe::egui::InputState) -> Vec<&'static str> {
    let mut out = Vec::new();
    let mut taken_keys: Vec<Key> = Vec::new();
    for (mods, key, line) in TIMELINE_KEYS {
        // One line per key: Shift+A wins over A, Cmd+Shift+V over Cmd+V.
        if taken_keys.contains(key) {
            continue;
        }
        if input.consume_key(*mods, *key) {
            taken_keys.push(*key);
            out.push(*line);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_animation_action_is_in_the_input_map_with_its_chord() {
        for (name, line, chord) in ACTIONS {
            let found = crate::input::ACTIONS.iter().find(|(a, _)| a == name);
            assert_eq!(found.map(|(_, c)| c), Some(chord), "{name}");
            assert!(line.starts_with("operandi."), "{line}");
        }
    }
}
