# Shared form dialogs: GUI check (UIC-80..99, UIC-125)

For Jason or Codex. QA build from branch `dialog-component`. Use the QA
app's own data folder.

1. New mesh, Polygon mode, select two polygons, click **Unify...**. A small
   window titled "Unify" opens in the middle of the main window: one
   "Force Unify" checkbox on the left (box first, text to its right), a thin
   divider, then OK (dim "Ret" at its right) over Cancel ("Esc"). No heading,
   no text under the fields, no bottom bar, no empty space (UIC-81..84, 87).
2. Press Return: Unify runs once (one Undo step), then a notice "N polygons
   eliminated" with OK alone. Return closes it.
3. Vertex mode, select points, **Set Position...**: Axis is three joined
   segments X | Y | Z (chosen one orange); Value shows "0 m" with small
   left/right arrows at its right end. Drag the arrows: the value changes
   (Shift coarser; Option or the physical Ctrl key finer). Type `2`, press
   Return once: the field takes 2 m and the dialog stays; Return again: OK
   (UIC-85, UIC-90).
4. Reopen Set Position: Axis, Value and the checkboxes come back as last
   OK'd. Quit and relaunch: still remembered (UIC-95). Cancel instead:
   nothing changes and the next opening still shows the last OK'd values.
5. Vertex **Merge...**: Distance is greyed (label and field, arrows greyed
   too) until Range is Fixed; nothing moves when it ungreys (UIC-92/93).
   The three unmeasured checkboxes are greyed with a tooltip saying why.
6. **Set Part...**: clear the Name: OK greys out with a tooltip; the small
   triangle at the field's right lists existing parts.
7. **Set Material...** (M): labels right-aligned against one edge, all
   fields the same width, Color is the usual swatch field.
8. Delete an item that has child items: the one Yes / No / Cancel
   question (Jason's deviation, GL-Q7): three buttons stacked on the
   right with Ret / Esc / Shift+Esc at their right; Escape answers No,
   Shift+Escape cancels. Every other dialog still says OK / Cancel.
9. File > **Close All...**: plain question, OK / Cancel (UIC-97). Edit
   something, File > Quit: "Save ... before quitting?" with OK (saves),
   Cancel, Discard.
10. Preferences > Mesh Items: the page is the panel mode of the same
   component: three darker section bars with triangles (fold one, reopen
   Preferences and relaunch: still folded), labels | small circle | field.
   Change a value: its circle fills; set it back: empty again.
   "Auto Create Item Transforms" is greyed, not hidden.
11. Mouse over any label or field: its tooltip explains it (UIC-82/119).
