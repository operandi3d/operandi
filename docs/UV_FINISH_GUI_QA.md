# UV tools finish: GUI check

Branch `claude/project-thread-ll5iqr`. Specs:
`specifications/uv-editing-2026-11-16`, `specifications/uv-2026-11-16`.
Use the QA app from `~/Developer/Operandi-Builds` with its own data
folder. Claude can't click in Operandi, so this pass is for Jason or Codex.

Set-up: Create a Box (Segments 2), switch to the UV layout (UV tab in the
layout bar).

1. **Project.** UV tab > Project. The box gets Planar UVs at once, seen
   from the top; the UV view shows them. Set Projection Type to Cubic: six
   squares on top of each other. Barycentric: every face fills the tile.
   Cylindrical, Axis Y: one band round the box, no face stretched across.
   Settings Manual greys nothing but Seam Angle (not Cylindrical) and moves
   the projection with Size and Center. Space drops it; Cmd+Z undoes it.
2. **Project from View.** Front view, select the front face, UV tab >
   From View, click in the 3D view: the face's UVs look like the face does
   from the front. Orbit and click again: they change.
3. **Unwrap tool.** Select the box's top rim and one vertical edge (Edge
   mode), UV tab > Unwrap: the cross layout shows at once. Drag left and
   right in either view: Iterations follows and the layout re-solves.
4. **Relax.** UV tab > Relax, Mode Spring, drag right: the islands smooth
   out. Boundary Locked: borders stay put. A Linear falloff limits it.
5. **Unwrap & Relax.** One click runs both; turning off Relax gives the
   plain unwrap.
6. **Element Move click.** UV view, W then Element Move (Deform tab), click
   a UV point without dragging: a teal U arrow and yellow V arrow appear on
   it. Drag the teal arrow: the point slides in U only.
7. **Soft Drag in UV.** Deform tab > Soft Drag, press and drag in the UV
   view: what's inside the circle moves, fading to its edge. Right-drag
   sizes the circle.
8. **Jitter on UVs.** Deform tab > Jitter in the UV view: Range Z and
   Enable Z are greyed; dragging right scatters the UVs; the box doesn't
   move.
9. **Slide's UV Map.** Edge mode, select an edge loop, Slide, set UV Map to
   Texture in Tool Properties: dragging slides the UVs, not the points.
10. **Align and Pack.** With two islands: U Min lines up their left sides;
    Orient Pieces squares a turned island; To Origin moves it to 0,0;
    Texel Density... Apply Individually 512 resizes each island.
11. **Split and Sew.** Select a shared UV edge, Shift+Split: the islands
    come apart. Select it again, Sew: they join halfway. Move and Sew on a
    far island brings it to the seam.
12. **Utilities.** Border selects every island outline; Error UVs... with
    Flipped finds a mirrored island; Delete then Unmapped selects that
    polygon; Copy, select another polygon, Paste puts the UVs there;
    Rotate turns a quad's texture 90°.
13. **Sub-D speed.** A Sub-D sphere (Level 3) in the UV layout with the
    3D view's Checker on: Move in the UV view stays smooth.
14. **Cut Map.** Select the box's top ring of edges, Cut Map > Add Seam:
    Show UV Seam (3D view's Display) draws them pale lavender. Clear the
    selection, Unwrap Tool (in Cut Map): it cuts there. Seam Marker with
    one edge of a ring selected marks the whole ring.
15. **UV Symmetry.** UV view header Sym: U. Move one UV point on the left
    of 0.5: its partner on the right moves mirrored; the dashed line shows
    at 0.5. Center welds UVs sitting on the line.
16. **Symmetrize.** A symmetric mesh with a wrecked -X half: Symmetrize...
    (X, Positive to Negative, U) copies the +X half's UVs back mirrored.
17. **Overlays.** Flipped tints a mirrored island yellow; Coverage shows
    lower right and updates when a drag ends; zooming out shows 1002,
    1011... in their tiles.
18. **Pack regions.** Pack UVs... with Nearest, Polygon Tag (Material),
    By Item, Background Constraint and Pack UVs to New Map each do what
    their tips say.
19. **Atlas.** Project > Projection Type Atlas on a sphere: many small
    charts fill the tile. Atlas 2 with Segments 4: fewer, larger charts;
    Segments and Iterations change them live.
20. **Peeler.** A tube (Edge mode), select one vertical line of edges, UV
    tab > Peeler: an even strip; Uniformity 0 % follows the 3D spacing.
21. **Export and convert.** Texture > Export UVs to SVG...: the file opens
    in a browser as the layout. Convert UVs to Mesh makes a flat "<name>
    UVs" item. Transfer UVs with two identical meshes selected (source
    first) copies the UVs.
22. **Box Transform.** UV tab > Box Transform with an island selected: a
    yellow box round it. Drag inside to move (Shift snaps its corner to the
    grid), drag the blue circle to turn (Ctrl steps 15°), drag the cross to
    move the pivot, right-drag a side to stretch from the opposite side.
23. **Split and Sew brush.** UV tab > Brush, brush over a shared UV point:
    it splits. Shift-brush over it again: it joins. Right-drag sizes the
    brush; click Brush again to stop.
24. **Relax pins.** Relax, Interactive on, Iterations 10: click two UV
    points (blue squares), drag one: the island stretches between them.
    Ctrl-click removes a pin.
25. **Icons.** The UV tab and the Edit tab's UV group show the new icons.
