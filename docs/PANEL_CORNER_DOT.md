# Panel corner dot (panel-corner-dot, 2026-11-06)

Built from the coordinator's behaviour summary (PANEL-DOT v1; the spec file
itself isn't in `specifications/`). Code: `crates/operandi-app/src/panel_dot.rs`
(the dot, its drags and menu), `crates/operandi-layout` (swap, dock beside,
tear off at a point, copies, linked quad, divider lock). Tests:
`panel_dot_tests.rs` (16) and the layout crate (18).

## What works

- **The dot.** A 5-point shaded dot at the far left of every panel's header:
  each 3D view (left of its header buttons), the Toolbox (a thin strip at its
  top), Items and Properties (left of the first tab), in the main window and
  in floating windows. Amber on the active panel, dark grey on the others
  (Preferences > Display > Colors > Interface: Active Panel Dot, Panel Dot).
  A press or plain click makes the panel active.
- **Hover.** The panel's whole border is outlined 1 point in the accent
  colour while the pointer is on its dot, and during its drag.
- **Drag a view's dot** (a few points of movement starts it; the pointer is a
  closed hand with a small hollow rectangle beside it):
  - over another view, five zones show (a middle half and four edge
    quarters, drawn as a frame), the one under the pointer filled with
    translucent accent where the panel will land;
  - centre: the two views swap places (each place keeps its size);
  - edge: dock beside: the target splits at half on that side, the dragged
    view takes that half, the hole it left closes;
  - anywhere outside every panel: tear off into a new OS window, top-left at
    the release point, the view's size, titled with the view's name;
  - a view alone in a floating window, dragged outside: a copy opens in
    another window there (the window can't be left empty); dropped on a
    view, it docks and its window closes;
  - Esc during the drag cancels: nothing changes.
- **Shift+drag**: a copy (same view type, angle, zoom and display settings)
  goes where the drag ends: a new window outside panels, or docked on an
  edge zone. The original stays.
- **Ctrl+drag**: the view splits at half in the drag's direction (mostly
  sideways: side by side; mostly up/down: stacked), the copy on the side the
  drag went; the copy becomes active. No drop zones.
- **Right-click the dot**: Duplicate (copy at the right of the window's
  layout), Copy (copy in a new window near the panel), Detach (= View > Tear
  Off View: the view's own window near where it was, at its size), Delete (=
  View > Close View) / Maximize or Restore (= Maximise) / Split Horizontal (=
  View > Split View), Split Vertical, Split Quad (four equal views, copies of
  it, one linked cross of dividers; the bottom-right one active). Greyed:
  Delete and Detach for the main window's last view (whose drags only
  Ctrl-split), Delete and Detach for a view alone in a floating window,
  Delete for a maximised view, Maximize for a view alone in its group.
- **Linked quad.** Split Quad and Maximise's Four Views both build a 2 x 2
  grid: dragging the vertical divider moves both rows' together, the
  horizontal one both columns'.
- **View > Layout**: Lock Thumb Dragging (no dot drags; the menu still
  works) and Lock Divider Dragging (no divider between views or side
  panels resizes). Both off by default, saved in `layout-locks.json`.
- **Floating view windows** are normal OS windows; closing one discards its
  views, no prompt.
- **Side panels** (Toolbox, Items, Properties) aren't in the layout yet:
  their dots activate and outline; dragged outside every panel (or Detach)
  the panel floats at the release point; a floating one dragged onto the
  main window docks back in its side panel. See PANEL-Q01.

## Changed behaviour

- View > Split View now splits the active view at half (the copy on the
  right, active) instead of adding a view at the right of the whole layout.
- View > Tear Off View opens the window near the view at the view's size
  (was a fixed 640 x 480 wherever the system put it).
- View > Close View is greyed for a view alone in a floating window or a
  maximised view.
- Closing a floating view window discards its views (it used to dock them
  back).

## GUI checks (for Jason)

| Check | Expected |
|---|---|
| PD-01 Dot | Every view, the Toolbox, Items and Properties show a small dot at the top left. The active view's is amber; click another view's dot: that one turns amber. |
| PD-02 Hover | Hover a dot: the panel's border lights up orange. |
| PD-03 Swap | Split View, then drag the left view's dot into the middle of the right view: they trade places. |
| PD-04 Dock beside | Drag a view's dot to the bottom edge of another view: the zones show, the bottom half fills; release: the view sits under it, half each. Try left, right and top. |
| PD-05 Tear off | Drag a view's dot onto the toolbar or the desktop and release: a window opens with its top-left at the pointer, same size; the main views fill the hole. |
| PD-06 Dock back | Drag that window's dot onto a main view's edge: it docks, the window closes. |
| PD-07 Cross-window | With a floating view window over the desktop, drag a main view's dot over it: its zones show there too. |
| PD-08 Esc | Start a dot drag over another view and press Esc before releasing: nothing changes. |
| PD-09 Shift / Ctrl | Shift+drag a dot outside: a copy floats, original stays, same view type and zoom. Ctrl+drag a dot down: the view splits, stacked. |
| PD-10 Menu | Right-click a dot: the menu opens at the pointer (near a screen edge it flips to stay on screen). Try each entry; on the only view, Delete and Detach are grey. |
| PD-11 Quad | Split Quad, then drag the vertical divider in the top row: the bottom row's moves with it. |
| PD-12 Locks | View > Layout > Lock Thumb Dragging: dot drags do nothing, the menu still works. Lock Divider Dragging: dividers don't move. Relaunch: both still ticked. |
| PD-13 Side panels | Drag the Items dot onto the desktop: Items floats there. Drag its dot back over the main window: it docks. |
| PD-14 Close window | Close a floating view window with its close button: its view is gone, no prompt. |
