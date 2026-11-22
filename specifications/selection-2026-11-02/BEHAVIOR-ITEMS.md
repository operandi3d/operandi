# Items: parenting and multi-item editing (SQ-008 part)

Setup for parenting: mesh "Parent" at (1,0,0) rotated 90° about Y; mesh
"Child" at (0,0,2), no rotation; both unparented.

**SEL-IT-01 Parent without keeping position.** Parenting Child to Parent with
"in place" off keeps Child's local transform (0,0,2), so its world position
becomes (3,0,0).

**SEL-IT-02 Parent keeping position.** With "in place" on, Child's world
position stays (0,0,2); its local transform becomes position (-2,0,-1),
rotation -90° about Y.

**SEL-IT-03 Unparent.** From either state, unparenting with the same
"in place" setting left Child at local and world (0,0,2).

**SEL-IT-04 Component edits act on every selected mesh item.** Two meshes
A (unit quad at x 0…1) and B (unit quad at x 3…4), both selected as items,
polygon 0 selected in each. Polygon Bevel (Inset 0.1, Shift 0.2) bevels both:
each goes from 4 points / 1 polygon to 8 / 5, and polygon 0 stays selected
in each.

**SEL-IT-05 Unselected items are not edited**, even with selected
components. With only A selected as an item (B still has polygon 0
selected), the same Bevel changed only A (8 → 12 points, 5 → 9 polygons);
B stayed at 8 / 5.
