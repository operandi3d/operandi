//! The User Guide's Reference > Keyboard shortcuts page is written from the
//! factory key map (`input::ACTIONS`), never by hand (guide RULES.md). This
//! test fails when the page and the key map differ; run it with
//! OPERANDI_BLESS_GUIDE=1 to write the page again:
//!
//!   OPERANDI_BLESS_GUIDE=1 scripts/check-fast.sh guide_shortcuts
use crate::input;

const PAGE: &str = "../../docs/guide/src/reference/keyboard-shortcuts.md";

/// A saved chord as the guide writes it. The guide writes **Ctrl** for the
/// key a Mac calls **Cmd** (Getting started), so a saved "Cmd" reads
/// "Ctrl"; a saved "Ctrl" is the real Control key on every system and
/// reads "Control".
pub(crate) fn guide_chord(chord: &str) -> String {
    chord
        .split('+')
        .map(|part| match part {
            "Cmd" | "Command" => "Ctrl".to_owned(),
            "Ctrl" | "Control" => "Control".to_owned(),
            "ArrowUp" => "Up Arrow".to_owned(),
            "ArrowDown" => "Down Arrow".to_owned(),
            "ArrowLeft" => "Left Arrow".to_owned(),
            "ArrowRight" => "Right Arrow".to_owned(),
            "NumpadDecimal" => "Numpad .".to_owned(),
            p if p.starts_with("Numpad") && p.len() > 6 => format!("Numpad {}", &p[6..]),
            "`" => "` (backtick)".to_owned(),
            "|" => "\\|".to_owned(),
            p => p.to_owned(),
        })
        .collect::<Vec<_>>()
        .join("+")
}

fn page() -> String {
    let mut out = String::from(
        "<!-- help-ids: operandi.inputMap ; template: concept ; since: 0.1.0 -->\n\
         <!-- Written by crates/operandi-app/src/commands/guide_shortcuts_tests.rs from the \
         factory key map. Don't edit by hand: run \
         OPERANDI_BLESS_GUIDE=1 scripts/check-fast.sh guide_shortcuts -->\n\
         # Keyboard shortcuts\n\n\
         These are the keys Operandi starts with. You can change any of them in \
         the **Input Map** (**Edit > Input Map…**, or **Input Map…** under \
         **Input** in **Edit > Preferences…**); the list there also shows \
         every action without a key.\n\n\
         - **Ctrl** is the **Cmd** key on a Mac, as everywhere in this guide.\n\
         - **Control** is the Control key on every system, a Mac's too.\n\
         - **Alt** is the **Option** key on a Mac keyboard.\n\
         - Some keys appear on more than one row. They do different things \
         depending on the selection mode, the tool or the window under the \
         pointer: **B** is **Bevel** in Polygon mode, **Edge Bevel** in Edge \
         mode and **Vertex Bevel** in Vertex mode.\n\n\
         | Action | Key |\n|---|---|\n",
    );
    for (action, chord) in input::ACTIONS {
        if chord.is_empty() {
            continue;
        }
        out.push_str(&format!(
            "| {} | **{}** |\n",
            // A pie's line reads as its name ("Pie: View Direction").
            crate::pie_menu::shipped_title(action)
                .unwrap_or(action)
                .replace('|', "\\|"),
            guide_chord(chord)
        ));
    }
    out
}

#[test]
fn guide_shortcuts_page_matches_the_key_map() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(PAGE);
    let now = page();
    if std::env::var_os("OPERANDI_BLESS_GUIDE").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &now).unwrap();
        return;
    }
    let saved = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        saved == now,
        "docs/guide/src/reference/keyboard-shortcuts.md differs from the key map; \
         run OPERANDI_BLESS_GUIDE=1 scripts/check-fast.sh guide_shortcuts"
    );
}

#[test]
fn guide_chords_read_as_the_guide_writes_them() {
    assert_eq!(guide_chord("Shift+Cmd+Z"), "Shift+Ctrl+Z");
    assert_eq!(guide_chord("Ctrl+Backspace"), "Control+Backspace");
    assert_eq!(guide_chord("Numpad1"), "Numpad 1");
    assert_eq!(guide_chord("NumpadDecimal"), "Numpad .");
}
