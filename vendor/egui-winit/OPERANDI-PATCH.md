# Operandi's patched egui-winit

This is egui-winit 0.33.3 from crates.io (by Emil Ernerfeldt and the egui
contributors, MIT OR Apache-2.0), with the changes in `operandi.patch`.

**Why:** egui has a single key for each digit, so egui-winit folds keypad 1
and main-row 1 into the same `Key::Num1` and throws away winit's
`KeyCode::Numpad1` / `KeyLocation::Numpad`. Operandi binds them to
different things (keypad = views, main row = selection modes).

**What the patch does** (`on_keyboard_input` plus a new `operandi_keys`
module at the end of `src/lib.rs`):

- Every digit and decimal-point key event is recorded with whether its
  scan code is the keypad's. The app drains it each frame with
  `egui_winit::operandi_keys::take_numbers()`.
- Keypad Enter presses are counted (`take_keypad_enters()`).
- Keypad digits stay digits with NumLock off (where the layout would make
  them End, arrows, Delete and so on).
- Cmd+V (Ctrl+V elsewhere) always sends `Event::Paste`, with empty text
  when the system clipboard holds no text (`paste_text`). Upstream dropped
  the key then, so Paste Selection never ran for copied polygons unless
  some text happened to be on the clipboard (2026-11-10). Text fields
  ignore an empty paste.

**Upgrading eframe/egui:** copy the new egui-winit release here, re-apply
`operandi.patch` (`patch -p0 src/lib.rs < operandi.patch` from this
folder, or by hand), and check `cargo test -p egui-winit`. Drop this copy
if egui ever reports the key location itself.

**Stuck I-beam on macOS (2026-11-21):** `set_cursor_icon` now calls
`operandi_cursor::unstick_ibeam()` on frames where egui wants the arrow and
the arrow is already what we last set. If macOS is still showing the
I-beam (AppKit sometimes fails to apply winit's cursor rect after moving off
a text field), it sets the arrow directly. Adds a macOS-only
`objc2-app-kit 0.2.2` dependency with the `NSCursor` feature, the same
version winit uses. Test: `cargo test -p egui-winit operandi_cursor`.

**Release outside the window (2026-11-21):** upstream drops a mouse button
event when it has no pointer position, which it forgets on `CursorLeft`. A
drag that left the window and was released before any further move then
never reported its release, so egui (and the app) thought the button was
still down. The state keeps `last_pointer_pos_in_points` and a release
falls back to it. Presses still need a current position.
