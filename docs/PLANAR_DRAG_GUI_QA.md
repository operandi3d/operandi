# Planar texture placement: GUI checks

For Jason or Codex (Claude can't click Operandi). Use the QA build named in
the report (`Operandi planar-drag <commit>.app`, launched with
`OPERANDI_CYCLES=1` for the Cycles line) on a throwaway scene. Built to the
Planar spec ("just do the modo way"): no dedicated drag; the ordinary item
tools place the texture. Choices: PD-Q01..Q14 in SPECIFICATION_QUESTIONS.md.
Each line: do this → expect that.

Setup: a Unit Plane (or cube) at the origin. Shading tab, Add Layer >
Image Map, pick any picture. Select the layer's locator row, Texture
Locator tab: Projection Type Planar, Axis Z. Look from the front (−Z).

1. Select the Image Map layer in the Shading tab, no tool active → the
   locator's plane draws in the selected-item colour, 1 px: the square at
   Size, a short line from the centre along the projection axis (a quarter
   of the smaller side), and a small tick on the middle of the top (+V)
   edge pointing up. Deselect the layer → the plane goes (unless Show
   Texture Locators is on, which draws it in the locator colour).
2. Select "Image Map (Texture)" in the Items list instead → the same
   drawing.
3. Press W (Move) on the locator → the usual Move handle at its centre;
   dragging it moves the square and the image live, and the Texture
   Locator tab's Position follows. Release, Cmd+Z → back in one step; Move
   stays active.
4. R (Scale): drag the X handle → the square and image widen about the
   centre; Size X follows. Drag it down to nothing → Size X stops at
   0.001 m, never zero or flipped. E (Rotate) → the square, tick and axis
   line turn with it.
5. In the Texture Locator tab, type Size X 0 → it becomes 0.001.
6. Shading-layer target: with no tool active, click the Image Map layer
   row in the Shading tab → the Items list highlights "Image Map
   (Texture)"; press W → the Move handle sits at the square and moves the
   texture. Click the plane in the Items list → the tool goes to the plane
   and stays there. Click the layer row again (after another row) → the
   tool jumps back to the locator.
7. Polygon mode, plane selected: click the layer row → the
   Items list keeps the plane, the locator's plane still draws, and W
   moves the plane's polygons as usual. Back in Items mode: select two
   texture layers → nothing changes in the Items list. Set the
   projection to UV Map → selecting the layer still selects its locator;
   UV Map draws only the locator's small axes.
8. Axis X, look from the side → the tick is on the top edge pointing up
   (+Y), the axis line points along +X. Axis Y, look from the top → the
   tick points toward −Z, the axis line up along +Y.
9. World Coordinates off (default): move the plane item 2 m in X, select
   the layer → the plane draws on the mesh, where the image is, not at the
   origin. The Move handle stays at the locator's own position (PD-Q08,
   not built yet). Tick World Coordinates → the plane draws at the
   locator's world place.
10. Cycles (`OPERANDI_CYCLES=1`, viewport set to the render view): repeat
    lines 3 and 4 → the render follows during the drag and matches the
    viewport after release.
11. Save, reopen → Position and Size come back.
12. Type the command once (UIC-164): select the locator, press R (Scale),
    then in the command line type `transform.channel name:scl.X value:2` →
    the square doubles in width exactly as a Scale X-handle drag to 200 %
    would, Scale stays active, and one Cmd+Z puts Size back. Type
    `transform.channel name:scl.X` → the status line reports `scl.X = 2`.
    Type `select.item item:<id>` with the plane's id → the tool moves to
    the plane (`item.select <id>` does the same and logs that the name is
    deprecated).
