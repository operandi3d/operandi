# Statistics panel GUI checklist

Spec: development/specs/statistics-panel/BEHAVIOR-STATISTICS.md. Use the QA
app with its own data folder.

1. Layout > Windows/Palettes > Statistics opens a floating window
   (about 400 × 600). A panel group's "+" > Info > Statistics adds it as a
   tab.
2. Open a box: Vertices 8, Edges 12, Polygons 6. Open Polygons > By vertex
   count: "4 (quads)" shows 6.
3. In Item mode click "+" on "4 (quads)": the mode becomes Polygons, all six
   are selected, and every Sel cell (Polygons, 4, Material Default) shows 6.
   Cmd+Z undoes it in one step. Click "−": they are deselected.
4. Drag down the "+" column over 3 and 4: both rows are added.
5. Right-click a part row: Rename… (or click, pause, click again) renames it;
   Cmd+Z puts the old name back. Delete removes the tag; the polygons stay.
6. Right-click "3 (triangles)": Hide hides them and the counts drop; Unhide
   brings them back.
7. Open Checks on a bent quad: Not flat 1 (cells show "…" for a moment on a
   big mesh).
8. Big mesh (Create a 1M+ polygon sphere, open every group): drag a vertex;
   the view stays smooth, the counts catch up a few times a second and once
   at the end.
9. Type `operandi.statistics.count row:polygons.vertices:4` in the command
   line: the status shows "n (m selected)".
10. Quit and reopen: the groups you opened are still open.
