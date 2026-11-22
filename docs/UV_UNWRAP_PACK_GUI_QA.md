# UV Unwrap and Pack UVs — GUI check

Build: the QA app from branch `claude/project-thread-iwegbz`. Use the QA
data folder, not your own scenes.

1. Make a Box. On the **Edit** tab, in the **UV** group, click **Unwrap...**. The dialog shows
   Iterations 100, Relative Gap Size 0.2, Initial Projection Planar,
   Projection Axis Y, Seal Holes and Update Layout on, Method Conformal,
   UV Seam Type Selected Edges; Symmetrize greyed with a reason.
2. Switch to Edges, select the four top rim edges and the four vertical
   edges, open Unwrap... again, OK. One undo step "UV Unwrap". With a
   checker or any image texture on the box's material, the cap and the four
   sides plus the bottom show as two islands, the texture unbroken across
   the cross and cut at the rim.
3. Set Initial Projection to Use Current Map: Projection Axis greys with
   "applies only to Planar, Cylindrical or Spherical".
4. Undo: the UVs go back to the box's own.
5. **Pack UVs...** with the starting values (Stretch on, Orient off, Gaps
   0.2): OK, one undo step "Pack UVs"; the islands fill the tile with a
   small edge gap and the cap tucked into the cross's empty corner.
6. In Polygons, select the cap only and Pack UVs again: only the cap moves;
   the cross stays put.
7. Type `uv.pack orient:true` on the command line: it runs with Orient on.
8. **UV view.** View > UV View: the view shows the 0-1 tile with a grid
   every 0.1, labels 0 / 0.5 / 1, the axis and "1001" faintly in the
   middle. The header reads "UV: Texture". The unwrapped box shows two
   islands, the seam edges pale lavender.
9. Alt+drag pans; the wheel zooms about the pointer; Ctrl/Cmd+Alt+drag
   zooms; A frames all UVs; with a polygon selected, Shift+A frames it;
   . and , zoom in and out.
10. In Polygons, click a polygon in the UV view: it fills with the
    selection colour there and is selected in the 3D view too. Shift+click
    adds, Ctrl/Cmd+click removes, a click on empty space clears.
11. With one polygon selected, Select Connected (]) selects just its UV
    island, not the whole box.
12. Turn on Overlap: after moving one island over another (or packing with
    Lock Stacked on a stacked pair), the overlapping polygons tint red.
    Turn on Distortion: polygons colour by UV size against 3D size.
13. The header's "3D" button (or View > UV View again) brings the 3D view
    back.

