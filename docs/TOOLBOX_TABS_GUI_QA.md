# Topology and Curve tabs — GUI check

Build: the QA app in ~/Developer/Operandi-Builds named in the thread.
"Left-click" = left mouse button; "press and hold" = hold the left button
on a button's corner triangle until its list opens.

1. The toolbox's vertical tabs end Polygon, **Topology**, **Curve**.
2. Left-click **Curve**: sections Tools (Bezier, Curve, B-Spline, Sketch,
   Retopo Sketch, Solid Sketch, Tube, Patch, Hair Guide) and Commands (Set
   Curves Color…, Freeze..., Align Winding, Make Open Curve, Make Closed
   Curves, Make Curve Fill, Edges to Curves, two offset flyouts), then the
   Convert Curves flyout and Bezier Tangent….
3. Press and hold **Open Curve Offset…**: the list shows the Trio too. Do
   the same on **Convert Curves**: six conversions; after picking one, the
   button's face shows it.
4. With nothing selected, Make Open Curve and Make Closed Curves are
   greyed (hover: the tooltip says why).
5. Draw an open Curve (left-click Curve, left-click four points in the
   Top view, Q), select it in Polygon mode, left-click **Open Curve
   Offset…**, type 0.1, Return: a copy 0.1 m to the left of the drawing
   direction, with sharp (mitred) corners.
6. Left-click **Topology**: sections Tools, Commands, Settings, and
   Modifiers inside Settings.
7. Add a background mesh (a sphere in another item, left unselected) and
   an empty mesh for the topology. Left-click **Topology Pen**: the
   Settings section's Constrain to Background lights and snapping turns
   on (status line).
8. Left-click **Topology Tweak**, then left-drag a point of the topology
   mesh: it moves and lands on the sphere.
9. Rows that aren't built (Contour, Instance Mirror, Merge Instance,
   Topology Mode, Offset, Selection Hit Size, Foreground Opacity) say so
   in their tooltip and do nothing.
