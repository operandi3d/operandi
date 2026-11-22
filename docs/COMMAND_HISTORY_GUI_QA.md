# Command History: GUI check (branch command-history)

For Jason, at the mouse, about 10 minutes. Use the QA app named in the
hand-back. Stage H (UIC-224..229, HIS-xx). Nothing else in the window
should look or behave differently.

1. **Looks the same.** Open the app in the Model layout. Compare with
   today's app: the right column, Items, Properties and the command entry
   at the bottom right sit where they always did. The only new thing is a
   thin line just above the entry that lights when you hover it (resize
   cursor).
2. **Open it.** Drag that line up: the panel opens with tabs Undos,
   History, Commands, Results, and the entry stays at the bottom. Drag it
   back down near the entry and let go: it snaps shut. Double-click the
   line: it opens (and closes again).
3. **The entry moved, nothing lost.** Type `poly.flip` with polygons
   selected and press Return: it runs as before. Press Up in the entry:
   the last line comes back; Down clears it (HIS-61).
4. **Undos tab (HIS-10..14).** Do three edits (Flip, a Bevel drop, Set
   Part). Each is a row, oldest at top: the Command column shows the step
   name, the next column the line that made it (e.g. `poly.setPart
   name:Road`). The bold row "Drag to undo/redo" sits under the newest.
5. Drag the bold row up two rows and let go: two steps undo at once; the
   rows below it go grey (R). Drag it back down: they redo.
6. Cmd+Z once, then make a new edit: the grey row is gone (HIS-14).
7. Click a row: its line appears in the entry, ready to edit. Double-click
   a row: that line runs again.
8. **Tool session (CMD-14, HIS-20).** Pick Move, drag the selection,
   change a value in Tool Properties, drop. Open the step's arrow in
   Undos: `tool.set TransformMove on`, the `tool.attr` lines (one per
   changed value, not one per mouse move), then the drop and `tool.set
   TransformMove off`. History shows the same lines in order.
9. **History tab (HIS-20..22).** It keeps everything, including the step
   you undid in 6, view changes (class I) and queries (Q). Type
   `history.max 3`, do a few things: only the last 3 lines stay. Type
   `history.clear`: it empties.
10. **Commands tab (HIS-30).** Open a category, find `poly.flip`, read its
    help in the next column. Double-click `poly.setPart`: its dialog opens
    (arguments missing). Fill it in and OK: History shows
    `poly.setPart name:<your name>`, not the bare line (CMD-14).
11. **Results tab (HIS-50).** Type `no.such.thing` and Return: Results
    shows it in red with the failure message. A typed query (`history.max
    ?`) shows its answer.
12. **Batch (CMD-40, UIC-227).** Type
    `cmds.batch label:Two c1:poly.flip c2:"poly.setPart Road"`. One Undos
    row "Two"; its arrow shows the two lines. One Cmd+Z takes both back.
13. **F filter.** On each tab, the F button shows the search row; typing
    narrows the list; F again brings the full list back.
14. **As a panel.** Use the "+" after a panel group's tabs: Utilities >
    Command History adds the whole panel as a tab (tabs, list and entry).
15. **Preferences > Defaults > Application.** Rows read Max Undo Levels
    (100), Group Selection Commands (off), Max Sub-Commands Recorded (100,
    no longer greyed). Undo works exactly as before with these defaults.
16. **UIC-239:** every control you used above shows in the Command
    History with its line (Undos drag = `cmds.undoTo n`, double-click =
    the row's line, Commands double-click = the command's name).
