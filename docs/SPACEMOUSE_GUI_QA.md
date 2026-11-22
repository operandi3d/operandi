# 3D mouse stage 1 — GUI checks

Build: `Operandi spacemouse-1 <sha>.app` in ~/Developer/Operandi-Builds.
Plug in the SpaceMouse first. With 3DxWare installed Operandi goes through
its driver (Device says "... (via 3DxWare)"); without it, raw USB.

How the settings combine: 3DxWare's own settings for Operandi (speed, axis
flips) apply first, then Operandi's Speed and Invert on top. Leave one side
at its defaults to keep it simple.

To log what the driver sends (for SM-Q05), start the app from Terminal:
`OPERANDI_SPACEMOUSE_LOG=~/Desktop/spacemouse.txt "<app>/Contents/MacOS/<exe>"`

1. Edit > Preferences > Input > 3D Mouse: Device says
   "SpaceMouse Compact connected" (or why not).
2. Open a scene with a mesh. Pointer over the view, twist the cap: the view
   orbits about its centre, smoothly, and stops when you let go.
3. Tilt forward/back: orbits over the top. Push sideways and lift/press:
   pans. Pull toward you / push away: zooms in / out.
4. Quad layout: pointer over another view, nudge the cap: that view moves
   and becomes active; the others stay.
5. A Top view: twisting turns it into a free view, as a mouse orbit does.
6. Speed 3×, Dead Zone 20 %, Motions = Rotate Only, each Invert: each
   changes the feel at once and is kept after a restart.
7. Enabled off: the puck does nothing; on again: Device reconnects.
8. Unplug the puck while moving: the view stops, Device says no 3D mouse;
   plug it back: it works again without a restart.
9. Click another app, then move the cap: Operandi's view doesn't move.
10. Buttons: click Button 1's field, press A (Frame All, or any
    shortcut); hover a view and press the puck's left button: it acts on
    that view as the key does. Esc while recording cancels without closing
    Preferences; Clear empties the row. Press a puck button with the page
    open: its row starts recording. Holding a button doesn't repeat.
    With 3DxWare, the button does only Operandi's shortcut, not 3DxWare's
    own action (SM-Q07).
11. With 3DxWare: its own Operandi settings (speed, invert) change the feel
    too; quitting 3DxWare and restarting Operandi falls back to raw USB.
12. Note any direction that feels backwards (SM-Q01).
