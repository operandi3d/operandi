# Symmetry for the modelling tools — GUI check (phases 1 to 3)

QA build with Symmetry X on (modes bar ▸ Symmetry: X) unless a step says otherwise.

1. **Pen.** Front view, Polygon mode, Pen. Click a point near the centre line (it snaps onto it), then three points to the right, ending near the centre line. A dimmed mirror outline follows while drawing. Drop: two polygons that share the centre-line edge; one Undo removes both.
2. **Pen crossing.** Draw a polygon with points on both sides: it is made once and the status bar says "Crosses the symmetry plane: not mirrored".
3. **Pen Make Quads.** Starting edge on the centre line, then clicks outward: a strip each side, joined at the centre.
4. **Solid Sketch (cartoon kid).** New mesh, Solid Sketch. Click on the centre line for the torso, again above it for the head, click the torso, then click to the right for an arm and further for the hand. The arm appears on the left too. Drag the left hand: the right one follows mirrored. Rotate/Twist an arm node: the other turns the opposite way. Middle-click a left node: both go. Drop, Tab: one smooth body, no seam crack.
5. **Solid Sketch, Symmetry off.** Turn Symmetry off, pick the sketch up again: the status bar says the sketch is symmetric about X; new nodes get no twin.
6. **Selection.** Double-click an edge loop on one side: its mirror loop is selected too. Double-click a polygon on one of two mirrored pieces: both pieces. Lasso one side: the mirror side is selected too.
6a. **Loop / Close Loop (bug reports #3, #4).** Polygon mode, click two neighbouring polygons on one side (four light up), press L: the loop and its mirrored loop. Again, then Shift+]: the closed loop on each side. Points mode: two neighbouring points, L: both point loops. Edges: one edge, Ring: both rings.
7. **Extrude / Extend / Slide.** Select both end polygons of a box, Extrude with the X handle: both ends grow outward. Same for Extend on two mirrored boundary edges, and Edge Slide on two mirrored loops.
8. **Loop Slice.** Click an edge on one side: a loop appears on the other side too.
9. **Symmetrize Polygons.** Pull one side of a symmetric mesh out of shape, Symmetry menu ▸ Symmetrize Polygons Negative: the right half is rebuilt from the left, joined at the centre.
10. **Bend.** Select both arms of a symmetric model (one side's selection adds the other), Bend, drag: both arms curl, mirrored (outward on both sides).
11. **Tear / Poly Haul.** Vertex Tear on two mirrored corners and drag sideways: both tear outward. Poly Haul in Tear mode on one corner: its mirror corner tears too.
12. **Sketch Extrude.** Select both end caps, draw an outward curve from one: the other cap follows the mirrored curve.
13. **Slices.** Polygon Slice a line across one side: the mirror cut appears too. Same with Edge Slice, Pen Slice and Axis Slice with an X Offset.
14. **Clone / Radial Array / Radial Sweep.** Select a symmetric pair of pieces, Clone with an X offset: the copies go outward on both sides. Radial Sweep a profile on each side: both turn, mirrored.
15. **Vertex Create.** Click points to one side: a mirror point appears for each; a click on the centre line makes one point.
16. **Symmetry Tool.** Pull a few points of one side slightly out of place (a few mm). Symmetry menu ▸ Symmetry Tool: yellow squares mark the unpaired points; drag right until they disappear and the side snaps back to mirror the other. Reverse moves the other side instead. With Symmetry off the menu entry is greyed out.
17. **Seam guide.** Symmetry X on a mesh: front view shows a thin red line down the centre; perspective shows a faint square outline through the middle. Turn Symmetry off: it goes.
18. **Toggle Symmetry.** Set Symmetry Z, then Symmetry menu ▸ Toggle Symmetry twice: off, then back to Z. Bind it to a key in the input map and try the key.
19. **Unpaired warning.** Pull one point of a symmetric mesh off its mirror position, select all, Move: yellow squares mark it and its old partner, and the status bar says "2 points have no mirror partner — Symmetrize or Symmetry Tool to fix".
20. **Seam snap distance.** Preferences ▸ Tool Handles ▸ Symmetry Seam Snap: set 30 px, then draw a Pen point well off the centre line but within 30 px: it lands on the line.
21. **Hide the symmetry plane.** Symmetry X on: the view's Display menu ▸ Show Symmetry Plane off hides the outline in that view only (other views keep it) and Symmetry stays on. Viewport Properties (O) ▸ Visibility ▸ General Visibility has the same checkbox.
22. **Lit Symmetry button.** The modes bar's Symmetry button is drawn in the accent colour while Symmetry is X, Y, Z or any other axis, like Snapping's; Off draws it plain. Toggle Symmetry lights and unlights it.
23. **Toggle with a tool open.** Symmetry off. Select a mirrored pair of points, open Move: the handle sits between them. Symmetry ▸ Toggle Symmetry (and a bound key): the tool stays open, the handle jumps to one side, and the next drag moves the pair mirrored. Toggle again and drag: both move the same way. Drop, one Undo per drag. Same with Tweak (one point, then its mirror partner too), an open Cone preview (doubles and halves), and Solid Sketch (new nodes get twins only while Symmetry is on).
