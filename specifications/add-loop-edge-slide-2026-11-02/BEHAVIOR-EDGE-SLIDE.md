# Edge Slide (numeric)

**ES-01 Percent moves each point along its own neighbouring edge by that
fraction of the edge's length.** G3, loop at x = 1, Percent 0.5: the four
loop points move to x = 0.5. On G3-irregular (neighbours 0.5 away in rows
0–2, 0.5 away in the shifted top row) the points go to x = 0.25 and, in the
top row, 1.0 → 0.75. Point and polygon counts unchanged. Selection after:
the same edges.

**ES-02 Direction.** Positive Percent moved the loop toward the -X
neighbours, negative toward +X (Percent -0.5 → x = 1.5). Setting Reverse X
was refused in this state ("only with a profile preset"). *(Which side is
"positive" in general, e.g. lower index or a handle side, is open: ES-Q01.)*

**ES-03 Distance interpolation** moves each point by that distance along
its edge: Distance 0.25 → x = 0.75 on G3 and on G3-irregular (0.5 → 0.25,
top row 1.0 → 0.75).

**ES-04 Reaching the neighbour merges (Merge on, default).** Percent 1.0:
the loop lands on the x = 0 column and merges with it: 12 points,
6 polygons (the column of polygons between them is removed). With Merge
off, Modo refused the setting ("only when Mode is Linear") and merged the
same way.

**ES-05 No clamping beyond the neighbour.** Percent 1.5 or Distance 1.5 →
x = -0.5, passing the neighbouring column (16 points, 9 polygons, folded
geometry). No message.

**ES-06 Border loop.** Loop at x = 0 (edges 0–4, 4–8, 8–12), Percent 0.5:
moves into the mesh to x = 0.5 (its only neighbour side).

**ES-07 A single edge** (5–9) with Percent 0.5 moves just its two points
to x = 0.5.

**ES-08** A point selection in Vertex mode with Percent 0.5 changed nothing.
