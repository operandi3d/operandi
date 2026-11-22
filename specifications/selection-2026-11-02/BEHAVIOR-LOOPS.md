# Loops and Previous/Next Loop

**SEL-LP-01 Loop from one polygon.** Grid4, polygon 5 → 1, 5, 9, 13 (the
column). From polygons 5 and 6 → 4, 5, 6, 7 (the row through both).

**SEL-LP-02 Next/Previous Loop for polygons** replace the loop with the
parallel loop beside it. Row 4–7, Next → 0, 1, 2, 3; then Previous → 4, 5, 6,
7.

**SEL-LP-03 Edge loops.** Grid4, edge 1–6 (vertical, x = 1), Loop → 1–6,
6–11, 11–16, 16–21. Next → the parallel loop at x = 0, which on this open
grid is the border, so the whole closed border (16 edges) is selected. Next
again → back to the x = 1 loop. Previous from the x = 1 loop → the border
again. With "additive" on, Next adds the border loop to the existing loop
(20 edges).

**SEL-LP-04 Point loops.** Points 6 and 7 → Loop gives 9, 8, 7, 6, 5 (the
row y = 1). Next → the 16 border points.
