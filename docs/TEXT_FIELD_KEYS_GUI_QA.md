# Text fields keep their keys — GUI check

Branch `text-field-keys`. Use the QA app named in the thread.

1. Select a polygon item, open its properties, click a number field (for
   example a Position value), type `10m`, press Return. The value becomes
   10 m and no Set Material window opens.
2. In the same panel, click a field and press Tab several times. Focus
   walks through the number and text fields top to bottom, skipping
   buttons and checkboxes, and wraps; Shift+Tab walks back. The polygons'
   Sub-D state does not change.
3. Open an options dialog with a text field and number fields (for
   example a Create preset save or Subdivide). Tab and Shift+Tab move
   between them; each value typed before Tab is kept.
4. Rename an item in the Items list (double-click its name), type `m`,
   `w`, `e`, `r`, `1`, Tab. The letters go into the name; no tool or mode
   changes.
5. Click in empty viewport space (no field focused) and press M and Tab:
   Set Material opens and Tab toggles Sub-D as before.
6. Open Create > Measure > Absolute Scaling (its own OS window). Click a
   field, type `10m`, press Tab: the value is kept, the next field opens,
   and Sub-D doesn't change. Same in the Snaps and Work Plane windows.
