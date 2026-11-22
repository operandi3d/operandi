# Select Between

Order of the resulting selection is as reported after the command: the picks
first, then the added elements.

**SEL-BT-01 One row between two picks.** Grid4, polygons 0 and 3 selected →
0, 1, 2, 3. Edges 0–1 and 3–4 → edges 0–1, 1–2, 2–3, 3–4.

**SEL-BT-02 Two picks not in one row select the whole rectangle.** Grid4,
polygons 0 then 10 → 0, 10, 4, 8, 1, 5, 9, 2, 6 (the 3×3 block). Picking 10
then 0 gives the same set (order 10, 0, 9, 8, 6, 5, 4, 2, 1).

**SEL-BT-03 More than two picks: only the last pair is connected.** Grid4,
polygons 0, 3, 15 → 0, 3, 7, 11, 15: the column from 3 to 15 was added, the
row between 0 and 3 was not. *(Confirm with other pick orders: SEL-Q-B1.)*

**SEL-BT-04 Points not in one row: nothing is added.** Grid4, points 0
(0,0) and 12 (2,2) → selection stays 0, 12.

> v2 draft: the general lower-index interpretation below is superseded by SEL-B2-03. Original measured examples and the one-route result remain valid.

**SEL-BT-05 Ties: one route is chosen, not both (SEL-Q-M1).** Ring8,
polygons 0 then 4 (two equal routes of three polygons) → 0, 7, 6, 5, 4. The
choice depends on pick order and indices, not on geometry:

| Picks (in order) | Result |
|---|---|
| 0, 4 | 0, 7, 6, 5, 4 |
| 4, 0 | 4, 3, 2, 1, 0 |
| 1, 5 | 1, 0, 7, 6, 5 |
| 5, 1 | 5, 4, 3, 2, 1 |
| 2, 6 | 2, 1, 0, 7, 6 |
| 3, 7 | 3, 2, 1, 0, 7 |

For polygons the route always leaves the first pick toward the next lower
polygon index (wrapping). Rotating every point by 90° about Y, keeping the
polygons, gives the same result. For points on Ring8 the route leaves the
first pick toward the next **higher** index: 0, 4 → 0, 1, 2, 3, 4; 4, 0 →
4, 3, 2, 1, 0; 2, 6 → 2, 3, 4, 5, 6. *(Whether the rule is really "lower
index" or "across the polygon's last edge", which coincide here, is
inferred: SEL-Q-B2.)*

**SEL-BT-06 Box, opposite faces.** Bottom then top → 0, 4, 1 (through face
4 only). Face 2 then face 4 → 2, 3, 4 (through face 3 only).
