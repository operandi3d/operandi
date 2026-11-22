# Selection falloff and Flex — GUI check

QA build from branch `claude/project-thread-ii6xy3`. Use the QA app identity
with its own data folder (`OPERANDI_DEV_DATA`).

1. New scene, Cylinder along Y with 10 segments. Polygon mode, select all
   polygons except the bottom ring and bottom cap.
2. Falloff menu ▸ Selection. Steps 5, Shape Ease-Out. Points tint by weight:
   none on the bottom border, full near the top.
3. Action centre Selection Border, Scale tool, drag in: the top tapers to a
   point, the bottom ring stays put (Kid 1 · 29:13).
4. Undo; Falloff None. Deform tab ▸ Flex (or Shift+Y over the view): the
   status names Flex, Tool Properties shows Transform's fields plus
   Selection Falloff Steps/Shape.
5. Select a band of polygons around the middle (two polygons, Shift+],
   Shift+Up a couple of times), Flex, rotate with the ring handle: the band
   bends smoothly; the polygons outside it don't move.
6. Right-drag left/right in the view: Steps changes in the field and the
   bend softens/hardens; the mesh doesn't scale.
7. Action Center ▸ Axis Element, click an edge: the handle lines up with it
   and Flex stays open.
8. Space drops; Falloff is back to None and the action centre to what it
   was. Undo removes the bend in one step; save, reopen: the bend is kept.
