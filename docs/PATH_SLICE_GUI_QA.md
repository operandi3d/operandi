# Edit tab waves 2 and 3: GUI checks

For Jason or Codex (Claude can't click Operandi). Use the QA build
`Operandi path-slice <commit>.app` in `~/Developer/Operandi-Builds`, on a
throwaway scene. Each line: do this → expect that.

## Edit tab

1. Edit tab, Slice section reads Polygon Slice, Curve Slice, Edge Slice,
   Pen Slice, Loop Slice, Primitive Slice. The three new rows are text only
   (no icon yet).
2. Below it an Axis Slice heading with Polygon Axis Slice, and Dicer and
   Julienne greyed (tooltip says wave 4).
3. Each new row starts its tool (lit) and shows Tool Properties; a second
   click drops it.

## Pen Slice

1. Cube (Segments 1). Top view. Pen Slice. Click four points inside the
   top face → a square is cut into the top **and** the bottom face (Axis Y
   cuts every surface the path passes through). Type Open and draw a line
   right across the cube → the two side faces it passes through are split
   top to bottom as well.
2. Tick Front Face Only → only the top is cut.
3. Type Open → the closing side goes; the open path inside one face cuts
   nothing (its ends are inside the polygon).
4. Drag a point → the cut follows. Cmd+Z → takes back the last click or
   drag; the tool stays on.
5. Drop (Q), Cmd+Z → the whole cut goes ("Undo Pen Slice").
6. Front view, start again: the first click fixes the plane; Axis Action
   Axis cuts the front and back faces.
7. A segment drawn close to horizontal snaps exactly level (World Axis);
   a third point near a right angle snaps square.

## Curve Slice

1. Grid (Plane, Segments 4). Top view. Curve Slice. Click three points
   across → a smooth curved cut through the grid.
2. Steps 6 vs 48 → coarser / smoother cut. Auto on → the step count
   follows the bends.
3. Closed on → the curve closes back to its first point.
4. Remove Last Point → the last point goes.

## Primitive Slice

1. Cube. Primitive Slice. Press on the top face and drag → a rectangle
   outline follows the pointer from the press corner; on release it is cut
   into the top and bottom faces (Lazy Apply).
2. Hold Ctrl (or Edit From Center) and drag → the press is the centre.
3. Click the +X side face → the shape moves there and cuts through both X
   sides.
4. Type Circle, Radius 0.3, Sides 12 → a 12-sided round cut. Ellipse uses
   Half Size X/Y.
5. Corner Radius 0.1 → rounded corners (Corner Segments per corner).
6. Clone X 3, Clone Gap X 0.05 → three shapes in a row; Center on →
   centred on the place.
7. Operation Stencil, Material "Red" → the inside gets the Red material.
   Operation Core → only the inside stays. Operation Tunnel on the cube's
   top → square holes in the top and bottom, no walls between them.
8. Lazy Apply off → the cut follows the drag live.
9. Select Sliced Edges on → the new edges join the "Sliced Edges" edge set
   (Select tab, selection sets).

## Polygon Axis Slice

1. Grid (Segments 4), Polygon mode, select the middle four polygons.
   Polygon Axis Slice → nothing is cut yet (Numbers start at 0).
2. Click the right arrow of Number X three times → three evenly spaced X
   cuts across the selection only; the left arrow takes them back off and
   stops at 0. Dragging the arrows steps whole numbers too.
3. Number Z 1 → one Z cut across the middle as well.
4. Drag left/right in the view → the cuts slide together; one pushed past
   the edge comes back in at the other side.
5. Slice Mode Size, Size X 0.25 m → cuts every 25 cm from the box's edge.
6. Hold Slice on → the cuts disappear while you change values; off → back.
7. Drop, Cmd+Z → one "Undo Polygon Axis Slice".
