# Layouts stage 6: palettes as layouts

Behaviour only.

Stage 6. Palettes become layout files shown in their own floating windows.
Rules landing: UIC-205, UIC-206, UIC-207.

- **LAY-50:** A palette is a layout file in its own floating window (its own
  OS window, the existing floating-panel rule). It opens and closes with one
  toggle: the same button or menu item opens it, and pressing it again closes
  it. Each palette exists once; opening it again brings the open one forward.
- **LAY-51:** Palettes remember size and position and reopen where they were
  (UIC-73). Default sizes, in points: UV Editor 800x600; UV View palette
  900x600; Preset Browser 900x600; a toolbox-set palette 260x650; Items,
  Properties, Shading, Images, Groups, Channels and Vertex Maps palettes
  400x600 (Properties 350x700).
- **LAY-52:** Palettes belong to the app, not to a layout: they stay open,
  where they are, when the layout changes. A viewport torn off a layout:
  behaviour on a layout switch is OPEN (LAY-Q2, a spec check is pending).
  Keep today's behaviour behind one clearly named switch, log it as
  provisional, and make the switch a one-line change.
- **LAY-53 / UIC-206:** Toggle Palettes: the ` (backtick) key hides every
  palette, and pressing it again shows them all.
- **LAY-54 / UIC-207:** Texture > Open UV Editor opens a floating UV Editor
  window: a UV view with the common UV tools down its right side.
  Double-clicking a UV map in the vertex map / UV map list opens the same
  window. It toggles like any palette. It's its own window (UIC-183).
- **LAY-55:** Palettes and pre-made floating windows are listed under
  Layout > Windows/Palettes, with Layout > Toggle Palettes there too
  ("Window menu" in UIC-184 means this menu).
- Today's Float Toolbox / Items / Properties become palettes: same look, now
  palette files. LAY-56 (a switcher bar per floating window) is out of scope.
- Check how Texture > Open UV Editor works today (it already exists from UV
  stage 1) and convert it rather than duplicating it.
