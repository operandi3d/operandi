# Delete key: GUI check (2026-11-21)

Delete is one app-wide rule. Over a 3D view, Delete and Backspace (the
Mac's delete key) delete the selection of the selection mode, together
with any points left unused. Shift+Backspace removes edges or polygons
and keeps their points. In Item mode they delete the selected items.

1. Polygon mode: select an isolated polygon, press delete. Expected: the
   polygon and its points go (Vertex mode shows nothing left). Cmd+Z
   brings it back in one step.
2. Same polygon, Shift+Backspace. Expected: the polygon goes, its points
   stay.
3. Edge mode: select one edge of a lone quad, press delete. Expected: the
   quad and its points go. Shift+Backspace: the quad goes, points stay.
4. Vertex mode: select a point on a cube, press delete. Expected: the
   point and the polygons using it go (a hole).
5. Click the Shader Tree tab so it is in front, then repeat 1 with the
   pointer over the 3D view. Expected: the same as 1 (this was the bug).
6. Item mode, two meshes, one selected: delete. Expected: it goes, no
   question.
7. Item mode, the only mesh selected: delete. Expected: the question
   "There is only one mesh item in the scene. Do you really want to delete
   the last mesh item?" with Yes (Ret), No (Esc), Cancel (Shift+Esc) and an
   In the Future dropdown. Yes deletes it; No and Cancel keep it.
8. Select the only mesh and a light, delete, answer No. Expected: the
   light goes, the mesh stays.
9. Repeat 7 with In the Future set to Always Yes, answer Yes. Expected: it
   goes; next time (New Mesh, delete) it goes without asking. Preferences,
   Defaults: Reset "In the Future" Messages makes it ask again.
10. Item mode, two meshes, both selected: delete. Expected: the question
    "The selection holds every mesh item in the scene. Do you really want
    to delete all the mesh items?". Yes deletes both; No keeps both.
