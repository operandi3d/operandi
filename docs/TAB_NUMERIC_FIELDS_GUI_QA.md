# Tab between numeric fields: GUI check (UIC-51)

For Jason or Codex. QA build from branch `tab-numeric-fields`.

1. New mesh, select a polygon, start Bevel. Click Inset in Tool Properties,
   type `0.1`, press Tab. Inset shows 0.1 m and the preview updates; the
   next field opens with its whole value selected.
2. Type a number straight away: it replaces the selected value. Shift+Tab:
   back to Inset, Shift+Tab again wraps to the last enabled field (the
   panel scrolls to show it).
3. Type `abc` in a field and press Tab: the field keeps focus with a red
   outline; nothing moves.
4. Item Properties (right panel): click Position X, type, Tab: Y opens, then
   Z. Tab never lands in the Tool Properties panel or on a button.
5. Preferences window: click a numeric value, Tab: the next numeric value
   in that window opens; checkboxes and menus are skipped.
6. With no field open, Tab still does what it did before (its shortcut, if
   one is bound).
