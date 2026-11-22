# Main window place: GUI check (UIC-73 [OP])

For Jason or Codex. QA build from branch `window-state`. The state lives in
`main-window.json` in the app's data folder.

1. Launch, move the window and resize it, quit (Cmd+Q). Launch again: same
   size and place.
2. Maximise (double-click the title bar or Option+green button), quit,
   launch: it opens maximised. Un-maximise: it returns to the size and
   place from step 1.
3. Fullscreen (green button), quit, launch: it opens fullscreen. Leave
   fullscreen: back to the step 1 size and place.
4. With a second monitor: put the window on it, quit, unplug the monitor (or
   switch it off in Displays), launch: the window opens on the main screen,
   fully visible, and fits it.
5. Delete `main-window.json` and launch: 1440x900 at the system's default
   place, as before.
