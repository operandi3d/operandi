# Layouts stage 6: palettes, GUI check

Spec: `specifications/layouts-2026-11-16/BEHAVIOR-LAYOUTS-STAGE6.md`
(LAY-50..55, UIC-205..207, the spec's LAY-Q2 answer). Use the QA app from
`scripts/bundle-dev.sh` with its own data folder (`OPERANDI_DEV_DATA`),
never your installed app. Tick each line; note anything odd next to it.
Start with a cube on the Model layout.

## Nothing else changed

1. [ ] Model and UV look as in the main QA build: same toolbox, panels and
       views. The Window menu now has only Diagnostics, Frame Time, MCP
       Bridge, Compact and Standard.

## Palettes

2. [ ] Layout > Windows/Palettes lists Toolbox, Items, Properties,
       Shading, Groups, Vertex Maps, Command History, Preset Browser, UV
       Editor, UV View, then Toggle Palettes with its key (`).
3. [ ] Choose **Groups**: a Groups window opens, about 400 x 600, with a
       check beside Groups in the menu. Choose it again: it closes.
4. [ ] Open Groups, move it and make it bigger, close it, open it again:
       same place, same size.
5. [ ] Open Groups, click the main window, then type
       `layout.window hash:groups open:true` in the command line: the
       Groups window comes to the front; there is still only one.
6. [ ] Choose **Toolbox**: the toolbox leaves the left column and floats
       as before (same look). Right-click a side panel header > Float in
       Its Own Window does the same for Items. Close both windows: they
       dock back.

## Layout switches

7. [ ] Open Groups and the UV Editor; place them. Click the **UV** layout
       tab, then **Model**: both windows stay open, where they were, the
       same size.
8. [ ] Split the 3D view (two views), zoom the right one, tear it off
       (View > Tear Off View). Switch to UV: the floating view stays, same
       place, camera and shading.
9. [ ] Switch back to Model: the main window has two views again (the
       slot has a fresh view looking like the torn one did), and the
       floating one is still there.
10. [ ] Close the floating view. Switch to UV and back to Model: it does
        not come back.

## Toggle Palettes

11. [ ] With Groups and the UV Editor open, press **`** (backtick, not in
        a text field): both disappear. Press it again: both come back
        where they were. Same with Layout > Windows/Palettes > Toggle
        Palettes.
12. [ ] Hide them with `, then choose Groups from the menu: everything
        comes back and Groups is in front.

## UV Editor

13. [ ] Texture > Open UV Editor: a UV view in its own window (about
        800 x 600) with Move/Rotate/Scale/Transform and the UV tools down
        its right side; the menu item is checked. Choose it again: it
        closes.
14. [ ] Open the UV Maps tab, double-click a map: the UV Editor opens on
        it (or comes forward if it's open).
15. [ ] Layout > Windows/Palettes > UV View: a plain UV view window
        (about 900 x 600), no tool strip.

## Restart

16. [ ] Leave Groups, the UV Editor and a torn-off view open; quit and
        relaunch the QA app: all three reopen where they were.
17. [ ] Preset Browser (Layout > Windows/Palettes or its key) opens and
        closes as before; a first-time open is about 900 x 600.
