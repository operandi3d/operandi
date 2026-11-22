# Select Symmetrical

**SYM-SEL-01** Select Symmetrical adds the counterpart of every selected
element in the current selection mode, keeping the original selection.
Setup: F1, Symmetry X. Vertex mode, points 9 and 8 selected → 9, 8, 6, 5
(new ones appended in that order). Edge mode, edge 8–9 → edges 8–9 and 5–6.
Polygon mode, polygon 3 → polygons 3 and 0. (SYM-O-04.)

**SYM-SEL-02** With Symmetry Off the command does nothing (selection
unchanged, no message).

**SYM-SEL-03** A point on the plane is its own counterpart (selecting 9 and 7
then the command gives 9, 7, 5).

**SYM-SEL-04** Points are matched by position, not by being part of a
polygon: an isolated point at (-2,0,0) pairs with (2,0,0).

> v2 draft: The general size-only interpretation below is superseded by SYM-B2-02; the original sample results remain valid.

**SYM-SEL-05** Matching tolerance is proportional to the mesh's size. On F1
(4 m wide) a counterpart off by 5×10⁻⁶ m along X still matches; off by
8×10⁻⁶ m it does not. Scaling the whole mesh by 100 or 0.01 scales both
thresholds by the same factor. *(Which size measure, e.g. bounding-box
diagonal or largest side, is open: SYM-Q04.)*

**SYM-SEL-06** With the Topology option on, a counterpart off by 10⁻⁵ to
10⁻² m along X is still found on F1 (matching uses connectivity).

**SYM-SEL-07** Picking an element by command (not by pointer) does not add
its counterpart, even with Symmetry on.
