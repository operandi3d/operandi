# Layouts v1.4, stage 2: user layouts, Revert, the full switcher bar

From research, 2026-11-16. Behavior only. Stage 1 is
`BEHAVIOR-LAYOUTS-STAGE1.md` in this folder.

## Scope

- Layout menu: New (copy box off), Save Layout, Save As, Rename, Delete
  (user layouts only), Revert with confirm, Clear, drag locks.
- Switcher bar extras: +, reorder, stars, starred-only, collapse,
  double-click revert (UIC-12..16); quick switcher Ctrl+Shift+Tab and the
  pie on Ctrl+` (UIC-18).
- Named pieces (Save Viewport / Save Viewport Group As) and the content
  arrow on panel headers (LAY-72, 73), so users can build a layout from one
  empty panel (LAY-74).
- User files sit over read-only shipped ones (UIC-194).

## Rules

UIC-12 A "+" after the last tab creates a new layout, added at the
right-hand end.

UIC-13 Drag a tab sideways to reorder; a thin insert line shows where it
will land.

UIC-14 Each tab has a star toggle. A "starred only" toggle on the bar hides
every unstarred tab.

UIC-15 Double-clicking a tab asks for confirmation, then resets that layout
to its shipped default. Never reset without asking.

UIC-16 A collapse control at the bar's left end hides the bar, leaving a
thin strip; clicking the strip brings it back.

UIC-18 Quick layout switcher: hold Ctrl+Shift and press Tab. A popup lists
layouts in most-recently-used order with the next one outlined. Each further
Tab (keys still held) moves the outline on. Releasing the keys switches to
the outlined layout. The same switcher opens from a Layouts button in the
bottom toolbar and from Layout > Recent Layouts…; Layout > Layouts lists
them all; a pie version opens on Ctrl+` . On Mac these are the real Ctrl
key.

UIC-192 Any frame or group can be saved as a named piece (Save Viewport As /
Save Viewport Group As, with a category) and reused in any layout; saving
the piece again updates every layout that uses it. Shipped layouts share one
left column and one right column.

UIC-193 Every panel header has the corner dot (the frame: split, delete,
detach, duplicate, save piece, revert, maximise, min header, locks) and the
content arrow (what the slot shows: any panel kind by category, Viewport
Group, Tabbed Viewport, panel settings). Together they build any layout from
one empty panel.

UIC-194 Shipped layouts and pieces are read-only; user layouts, pieces and
changed copies of shipped ones are files in the user's data folder.

UIC-199 Changes are remembered automatically. Save Layout, Save Layout As,
Rename, Revert (double-click the tab; asks first), Delete (user layouts
only; asks first), Clear Layout.

UIC-200 New layout asks for a name, with As Copy of Current Layout (off by
default).

LAY-32 Mac keys: Cmd+Shift+Tab and Cmd+` are taken by macOS, so on Mac the
quick switcher is real Ctrl+Shift+Tab and the pie is real Ctrl+`.

LAY-42 Reset to default: double-click the layout's tab, or Layout > Reset
Layout. A confirmation dialog first; never reset without asking. Reset drops
the user copy.

LAY-43 New layout: the + at the end of the bar, or Layout > New. A small
dialog asks for a name and has an As Copy of Current Layout checkbox.
Unticked gives a blank window with one empty panel to build from; ticked
copies the current one. The new tab goes at the right-hand end and becomes
current. The checkbox starts unticked.

LAY-44 Save Layout As: saves the current arrangement as a new layout under a
new name (same as New with the copy box ticked).

LAY-45 Rename: Layout > Rename Layout, or right-click the tab > Rename.

LAY-46 Delete: Layout > Delete Layout, or right-click the tab > Delete. Only
user layouts can be deleted; shipped ones can be reset but not deleted. Asks
first.

LAY-47 Clear Layout empties the current layout to one empty panel.

LAY-48 Layout > Lock Thumb Dragging and Lock Divider Dragging stop
accidental rearranging. Operandi already has layout locks from the
corner-dot work; they move to this menu and become per layout. (View >
Layout's two lock entries from stage 1 move here.)

LAY-72 Shared pieces: any frame, single panel or whole group, can be saved
under a name and a category (Save Viewport As, Save Viewport Group As) and
used in any layout. Save Viewport (Group) updates that named piece; Revert
puts a piece back to its saved version; Delete Saved Viewport removes a user
piece. Shipped layouts share one left-column group and one right-column
group, so changing it once changes them all.

LAY-73 Two controls on every panel header: the corner dot (frame): drag =
detach to a floating window; Shift+drag = copy to a floating window;
Ctrl+drag = split in the drag's direction; right-click menu = Duplicate,
Copy, Detach, Delete, the Save Viewport items, Revert, Maximise/Restore, Min
Header, Lock Width, Lock Height, Split Horizontal, Split Vertical. Dragging a
divider resizes (much of this exists from the corner-dot work; reuse it).
The content arrow at the header's top right: right-click lists the common
panel kinds first, then every kind by category; plus Viewport Group (nest a
sub-layout here), Tabbed Viewport (make this slot a tab group), that panel's
own settings, the corner-dot menu again, and the Layout menu items. Picking
a kind replaces what the slot shows.

LAY-74 A new empty layout (copy box off) starts as one empty panel; the user
splits it with the corner dot and fills each slot with the content arrow.

LAY-60 One small file per changed layout in the data folder; a separate file
holds tab order, stars, starred-only, collapsed state, MRU list and open
floating windows. Not in scene files.
