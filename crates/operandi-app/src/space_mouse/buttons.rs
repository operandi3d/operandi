//! Puck buttons (stage 2): each button presses the keyboard shortcut set
//! for it in Preferences > Input > 3D Mouse. The press goes in as an
//! ordinary key event at the start of the next frame, so it runs through
//! the same key dispatch as the keyboard: same action, same hovered-view
//! rule (UIC-70), same blocking while a dialog is open. No new commands.
use crate::Operandi;
use crate::native::NumberKey;
use eframe::egui::{Context, Event, Key};

/// Synthetic input for the next frame.
#[derive(Default)]
pub struct Pending {
    pub events: Vec<Event>,
    pub numbers: Vec<NumberKey>,
}

/// The key events (and, for digits, which row they came from) that
/// press and release `shortcut`. `None` when it isn't a shortcut.
pub fn shortcut_input(shortcut: &str) -> Option<(Vec<Event>, Option<NumberKey>)> {
    let (modifiers, key) = crate::input::chord(shortcut)?;
    let event = |pressed| Event::Key {
        key,
        physical_key: Some(key),
        pressed,
        repeat: false,
        modifiers,
    };
    // Digit shortcuts tell the keypad from the main row by the native
    // layer's report (input::chord): say which this one is.
    let digit = if key == Key::Period {
        Some(10)
    } else {
        key.name().parse::<u8>().ok().filter(|&d| d <= 9)
    };
    let number = digit.map(|digit| NumberKey {
        digit,
        keypad: shortcut.contains("Numpad"),
        pressed: true,
    });
    Some((vec![event(true), event(false)], number))
}

/// The buttons that went down between two button masks (0-based).
pub fn newly_pressed(before: u32, after: u32) -> impl Iterator<Item = usize> {
    let down = after & !before;
    (0..32).filter(move |i| down & (1 << i) != 0)
}

impl Operandi {
    /// Take the frame's puck button presses: with Preferences on the 3D
    /// Mouse page a press picks that button's row to record; otherwise it
    /// presses the button's shortcut.
    pub(super) fn space_mouse_buttons(&mut self, ctx: &Context, focused: bool) {
        let presses = std::mem::take(&mut self.space_mouse.presses);
        let on_page = self.show_preferences
            && self.preferences_ui.page == crate::preferences::Page::SpaceMouse;
        if !on_page {
            self.space_mouse.recording = None;
            self.space_mouse.keys_taken = false;
        }
        for button in presses {
            if on_page {
                self.space_mouse.recording = Some(button);
                ctx.request_repaint();
                continue;
            }
            if !focused || !self.space_mouse.prefs.enabled {
                continue;
            }
            let Some(shortcut) = self.space_mouse.prefs.buttons.get(button) else {
                continue;
            };
            if let Some((events, number)) = shortcut_input(shortcut) {
                let pending = &mut self.space_mouse.pending;
                pending.events.extend(events);
                pending.numbers.extend(number);
                ctx.request_repaint();
            }
        }
    }

    /// Hand the queued button presses to this frame's input (from
    /// `raw_input_hook`, before anything reads the keys).
    pub(crate) fn space_mouse_take_events(&mut self, events: &mut Vec<Event>) {
        events.append(&mut self.space_mouse.pending.events);
    }

    /// The queued presses' digit rows, after the native layer's own.
    pub(crate) fn space_mouse_take_numbers(&mut self) {
        let numbers = std::mem::take(&mut self.space_mouse.pending.numbers);
        self.native_numbers.extend(numbers);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shortcut_becomes_a_press_and_a_release() {
        let (events, number) = shortcut_input("Shift+F").unwrap();
        assert_eq!(events.len(), 2);
        match &events[0] {
            Event::Key {
                key,
                pressed,
                modifiers,
                ..
            } => {
                assert_eq!(*key, Key::F);
                assert!(*pressed);
                assert!(modifiers.shift && !modifiers.alt);
            }
            other => panic!("{other:?}"),
        }
        assert!(matches!(events[1], Event::Key { pressed: false, .. }));
        assert!(number.is_none());
        assert!(shortcut_input("Hyper+Q").is_none());
        assert!(shortcut_input("").is_none());
    }

    #[test]
    fn digits_say_whether_they_are_on_the_keypad() {
        let (_, number) = shortcut_input("Numpad5").unwrap();
        let number = number.unwrap();
        assert_eq!((number.digit, number.keypad), (5, true));
        let (_, number) = shortcut_input("5").unwrap();
        assert!(!number.unwrap().keypad);
    }

    #[test]
    fn only_buttons_that_go_down_count() {
        assert_eq!(newly_pressed(0, 0b11).collect::<Vec<_>>(), [0, 1]);
        assert_eq!(newly_pressed(0b01, 0b11).collect::<Vec<_>>(), [1]);
        assert_eq!(newly_pressed(0b11, 0b01).count(), 0, "release");
        assert_eq!(newly_pressed(0, 1 << 31).collect::<Vec<_>>(), [31]);
    }
}
