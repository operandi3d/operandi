# Draw point modes, repeat transform, drag snap — GUI check

QA build from branch `draw-edit-repeat-snap`.

1. Cube, Polygons, select its top. Rotate tool, type Angle 15 on Y, then
   middle-click in the view: it turns to 30°. Middle-click 22 more times:
   back where it started. Undo once takes back one 15° step.
2. Drop the tool, turn Rotate on again, middle-click: nothing happens.
3. Move tool: drag a little right, then middle-press and drag: it repeats
   the step and keeps hauling from there; release is one undo step.
4. Create ▸ Curve: click four points. Tool Properties ▸ Mode shows Add /
   Edit / Delete as joined segments (tooltip lists the gestures).
5. Mode Edit: clicking empty space adds nothing; drag a point to move it.
6. Mode Delete: hovering a point shows it red; clicking removes it and the
   curve re-forms. Undo brings it back. Middle-click deletes in any mode;
   Shift+middle-click deletes it and every later point.
7. Bezier Curve: Mode has Insert. Click on the curve: a new anchor appears
   and the curve doesn't change shape.
8. Pen: Mode row at the top. Delete removes a point; Add then puts the next
   point after the current one. Make Quads: deleting a point takes its
   quad's pair. Undo steps back one add, drag or delete at a time.
9. Tube: Mode row at the top of its properties. Click a point, then click
   empty space: the new point goes after it. Delete mode and middle-click
   remove points; Sides and Radius stay.
10. Snapping on (X), Vertex target: drag a Curve point toward a mesh
    point: a ring appears inside ~40 px and the point jumps onto it inside
    ~24 px, before release. Same for a Bezier handle end, a Pen point and
    a Tube point. Ctrl-drag stays on its axis. Hold X during a drag to flip
    snapping for that drag only.
