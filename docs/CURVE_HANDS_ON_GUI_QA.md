# Curve hands-on batch — GUI check

Build: the QA app in ~/Developer/Operandi-Builds named in the thread. Fresh
scene. "Left-click" = left mouse button (plain trackpad click);
"left-drag" = hold the left button and move; "right-click" = right mouse
button (two-finger trackpad click). Keys: 1 Vertex mode, 3 Polygon mode,
Q drops a tool, Space drops a tool, Esc cancels.

## Pen Extrude

Follow docs/PEN_EXTRUDE_GUI_QA.md. New since then: Start at Source is off,
so the walls start from where you click, and Tool Properties > Sweep
Effector has **Corner Scale** (on): at a bend the middle ring is wider
across the bend so the walls keep their thickness. Untick it with a
left-click: the middle ring shrinks back to the face's own size.

## Conversions

1. Curve tab > Draw: left-click **Curve**. Left-click four points in the
   Top view, then left-click **Closed** in Tool Properties to tick it.
   Press Q.
2. Press 3 (Polygon mode) and left-click the curve to select it.
3. Curve tab > Convert: left-click **Curve to Bezier**. A new item
   Converted_Bezier appears in the Items list; the Bezier tool comes on in
   Edit mode with the curve's knots and handles shown. The original curve
   is still there underneath.
4. Left-drag a knot (square) or a handle (round): the Bezier changes, the
   original doesn't. Press Q to finish.
5. With the Bezier selected (Polygon mode), left-click **Bezier to
   B-Spline**: another new item, Converted_Bspline, slightly inside.
6. Select nothing of the right kind (e.g. the B-Spline only) and hover
   **Curve to Bezier**: the button is greyed and its tooltip says why.
7. Cmd+Z after a conversion: the new item goes away in one step.

## Offsets

8. Select a closed curve (Polygon mode, left-click it). Left-click
   **Closed Curve Offset…**: a small dialog "Enter Offset Amount" with
   Offset 0 m. Type 0.1, press Return. A second curve appears 0.1 m
   inside the first, in the same item. Cmd+Z removes it.
9. **Closed Curve Offset Trio…** with 0.1: one curve inside and one
   outside; the original stays between them.
10. Draw an open Curve (Create > Draw > Curve, left-click three points,
    Q). Select it and use **Open Curve Offset…** with 0.1: a parallel copy
    to one side. **Open Curve Offset Trio…**: one copy each side.
11. **Closed Curve Offset…** on an open curve: the status line says to
    select closed curves; nothing changes.

## Bezier Tangent

12. On a Bezier curve, press 1 (Vertex mode) and left-click one knot.
    Curve tab > **Bezier Tangent…**: a dialog with Operation (Sharp,
    Smooth, Uniform), Tangent (Both, Out, In), Set Length (off) and Length
    (greyed until Set Length is ticked).
13. Sharp + Both, Return: the knot becomes a corner, each handle pointing
    at its neighbouring knot.
14. Bezier Tangent… again with Smooth: the two handles line up through the
    knot, each keeping its length. Uniform: lined up and the same length.
15. Tangent Out with Set Length ticked and Length 0.5: only the Out handle
    changes, to 0.5 m long.
16. Each run is one Cmd+Z step.
