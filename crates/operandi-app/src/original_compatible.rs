//! The "original-compatible" switch (Preferences > Defaults > UI,
//! `pref.value ui.original_compatible`). Where Operandi deliberately
//! behaves differently from the original application it is modelled on,
//! the switch brings back the original's behaviour, one registered
//! deviation at a time.
//!
//! To add a deviation: add an entry to [`DEVIATIONS`], then where the
//! behaviour is chosen ask `original_compatible::is_original("A1")`. The
//! Preferences row lists every entry, so the list is the one place to
//! look. Each entry arrives with its own behaviour spec.
use std::sync::atomic::{AtomicBool, Ordering};

/// One place Operandi differs from the original on purpose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Deviation {
    /// Stable id ("A1", "A2", ...), used by the code that asks.
    pub id: &'static str,
    /// What the switch changes, in one plain sentence.
    pub change: &'static str,
}

/// Every registered deviation, in id order.
pub(crate) static DEVIATIONS: &[Deviation] = &[
    Deviation {
        id: "A1",
        change: "Tool handles have a size in the scene: they shrink as you zoom in",
    },
    Deviation {
        id: "A4",
        change: "Empty backdrops show a dotted 1 m square",
    },
    Deviation {
        id: "A5",
        change: "In Perspective, backdrop pictures cover meshes",
    },
    Deviation {
        id: "A6",
        change: "Set Part starts with the last name it applied (else Default Part)",
    },
    Deviation {
        id: "A7",
        change: "On a Mac, only Cmd+click toggles list rows; Ctrl+click is a right-click",
    },
];

/// Mirrors `UiPreferences::original_compatible` (set by `apply`), so any
/// code can ask without reaching the app state.
static ON: AtomicBool = AtomicBool::new(false);

/// Turns the switch on or off (from the UI preferences).
pub(crate) fn set_on(on: bool) {
    ON.store(on, Ordering::Relaxed);
}

/// Whether deviation `id` should behave as the original now: the switch is
/// on and `id` is registered.
pub(crate) fn is_original(id: &str) -> bool {
    is_original_in(DEVIATIONS, ON.load(Ordering::Relaxed), id)
}

fn is_original_in(entries: &[Deviation], on: bool, id: &str) -> bool {
    let known = entries.iter().any(|d| d.id == id);
    debug_assert!(known, "original-compatible deviation {id} isn't registered");
    on && known
}

/// The Preferences row's note: what turning the switch on changes.
pub(crate) fn summary() -> String {
    summary_of(DEVIATIONS)
}

fn summary_of(entries: &[Deviation]) -> String {
    if entries.is_empty() {
        return "Nothing changes yet.".into();
    }
    let mut text = String::from("When on, these change:");
    for d in entries {
        text.push_str(&format!("\n• {}: {}", d.id, d.change));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST: &[Deviation] = &[
        Deviation {
            id: "T1",
            change: "Test handles shrink with zoom",
        },
        Deviation {
            id: "T2",
            change: "Test pies open at once",
        },
    ];

    #[test]
    fn a_registered_deviation_follows_the_switch() {
        assert!(!is_original_in(TEST, false, "T1"));
        assert!(is_original_in(TEST, true, "T1"));
        assert!(is_original_in(TEST, true, "T2"));
    }

    #[test]
    #[should_panic(expected = "isn't registered")]
    fn asking_for_an_unregistered_id_is_a_mistake() {
        is_original_in(TEST, true, "Z9");
    }

    #[test]
    fn the_preferences_note_lists_what_changes() {
        assert_eq!(summary_of(&[]), "Nothing changes yet.");
        assert_eq!(
            summary_of(TEST),
            "When on, these change:\n• T1: Test handles shrink with zoom\n• T2: Test pies open at once"
        );
        assert!(summary().starts_with("When on, these change:"));
    }

    #[test]
    fn ids_are_unique_and_described() {
        for (i, d) in DEVIATIONS.iter().enumerate() {
            assert!(DEVIATIONS[..i].iter().all(|e| e.id != d.id), "{}", d.id);
            assert!(!d.change.is_empty(), "{}", d.id);
        }
    }
}
