# Pen Make Quads GUI check (SPEC-PEN-MAKE-QUADS)

Use the QA build; your own scenes stay closed. Front view, Polygon mode,
a new empty mesh.

1. **PMQ-G01 Strip.** Pick the Pen, tick **Make Quads** in Tool
   Properties. Click two points side by side (the starting edge: a line,
   no polygon yet). Each later click adds a quad above the last, its
   second corner offset like the starting edge. The preview fills each
   quad.
2. **PMQ-G02 Drag.** Drag any strip point, including a partner point you
   didn't click: only that point moves.
3. **PMQ-G03 Ctrl triangle.** Ctrl+click: one point closes the strip's end
   with a triangle. A further plain click says "This strip ends with its
   triangle; Shift+click starts a new strip".
4. **PMQ-G04 Shift new strip.** Shift+click: the strip is added and a new
   strip starts (Make Quads still ticked); click its starting edge and a
   quad, then Space. Both strips face you, the new one is selected.
   Cmd+Z removes the second strip only, again the first.
5. **PMQ-G05 Greying.** With points captured, Make Quads greys out ("can't
   change mid-draft"). With Make Quads on, Wall Mode greys out; with Wall
   Mode on, Make Quads greys out. Type Vertices or Polyline greys Make
   Quads ("applies to the polygon types").
6. **PMQ-G06 Undo while drawing.** Cmd+Z mid-strip takes back the last
   click (its point and partner) without leaving the tool.
7. **PMQ-G07 Edge Join.** Draw two strips whose ends face each other and
   join the end edges with Edge Join: one band.
8. **PMQ-G08 Round a curve.** Sculpt or add a sphere, turn on Constrain to
   Background, and draw a strip round it from the front (starting edge
   pointing outward, then clicks round in a circle, both directions on
   separate strips; also with the sphere in SubD). Each new partner stays
   on the outside of the ring and on the surface, no quad folds or crosses;
   holding the click and dragging moves the new partner with it.
