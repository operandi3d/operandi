# Viewport split and tear-off — GUI checks

Build: the QA app named in the thread. Use a fresh data folder.

| Check | Do | Expect |
|---|---|---|
| VST-G01 Four directions | Hold Ctrl, drag the view's dot down 80 pt and release. Repeat right, left and up on new views. | Each time a new view appears on the side you dragged, half the size, with the same angle and zoom; the new one has the amber dot. |
| VST-G02 Preview | Ctrl+drag a dot and move around before releasing. | An insert line and a light fill show the half the new view will take, following the pointer. |
| VST-G03 Swing | Ctrl+drag down, then swing right, release. | A right split. |
| VST-G04 Esc | Ctrl+drag, press Esc, release. | Nothing changes. |
| VST-G05 Too small | Split side by side until views are narrow, then Ctrl+drag right on a narrow one. | Preview is red; release does nothing; the dot menu's Split Horizontal is greyed with a reason. |
| VST-G06 Dividers | Drag a divider hard toward a neighbour. | It stops when the neighbour is about 120 pt wide. |
| VST-G07 Delete in a row | Three views side by side, delete the middle one from its dot menu. | Its two neighbours share the space; nothing nests. |
| VST-G08 Detach | Dot menu > Detach on a view near the right edge of the screen. | A window opens where the view was, fully on screen, staying above the main window. Shortcuts (e.g. F for frame) work in it. |
| VST-G09 Split inside | In the torn-off window, Ctrl+drag the dot down. | The window holds two stacked views. |
| VST-G10 Undo | Add a cube, split, detach, dock back, then Cmd+Z. | The cube goes; the views stay as they are. The title shows no new unsaved mark from the layout changes alone. |
| VST-G12 Beside the workspace | Set a view to Camera, drag its dot off the main window onto the desktop (or a second monitor) and release. Move the window further out, then quit and relaunch. | It is its own OS window, fully outside the main window wherever you put it, and comes back there. Only a window that would hang off a screen's edge is nudged back onto that screen. |
| VST-G11 Unplugged screen | Tear a view off onto a second monitor, quit, unplug it, relaunch. | The window opens on the main screen near the main window. |
