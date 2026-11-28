<!-- help-ids: Preferences ; template: panel ; since: 0.1.0 -->
# Preferences

**Preferences** (**Edit > Preferences…**, **Ctrl+,**) holds the settings
that are yours rather than the scene's: they apply to every scene and are
kept between sessions. On a Mac it is also in the application menu,
**Operandi > Preferences…**.

![The Preferences window on its Input page](../images/panel-preferences.png)

## Pages

| Group | Pages |
|---|---|
| **Input** | **Remapping**, **Selection**, **Distance Units**, **Grid and Work Plane**, **3D Mouse** |
| **Display** | **Colors**, **Tool Handles**, **3D Information Overlays** |
| **Defaults** | **Application**, **Auto-Save**, **Mesh Items**, **UI**, **Tool Settings**, **Scene Save/Export**, **Default Tools** |
| Others | **File I/O**, **Rendering**, **Images & Painting** |

The **Input** page sets **Mouse Navigation** (which mouse gestures orbit,
pan and zoom; the rows under it show the current ones), opens the **Input
Map...** to change keys, and sets up a pen tablet.

## Free Orbit and Free Cursor

Two mouse options on the **Remapping** page, both off at first. Each
applies to every view.

- **Free Orbit**: orbiting up or down keeps going over the top and all the
  way round, instead of stopping just short of looking straight down or up.
  Past the top the view is upside down. A view looking through a camera or
  a light still stops.
- **Free Cursor**: while you drag in a view, the pointer jumps to the
  opposite edge when it reaches one, so the drag never runs out of room.
  It works for orbit, pan, zoom, tool handles and the drags that change a
  value; selecting, drawing and brush strokes stay under the pointer.

## Trackball Rotation

**Trackball Rotation** on the **Remapping** page, off at first, makes
orbiting turn the view freely about its own axes instead of around the
world's up direction. The first time you orbit a 3D view, a small note at
the bottom of the view asks **Try Trackball Rotation?**: **Turn On** turns
it on, **No Thanks** keeps the usual orbit. The note goes by itself after
17 seconds and never comes back.

## Common tasks

- **Change a setting**: pick a page on the left and change the row. It
  applies at once.
- **Put a page back**: **Reset Panel** at the bottom resets the current
  page; **Edit > Reset Preferences…** resets all of them.
- **Close**: **Close**, or **Esc**.

The dot before a row marks a setting saved with your preferences.

## Related

- [Keyboard shortcuts](../reference/keyboard-shortcuts.md)
- [Getting started](../getting-started.md): navigation presets
