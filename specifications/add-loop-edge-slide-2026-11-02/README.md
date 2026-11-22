# Add Loop and Edge Slide

**Status: v4 draft (partial); prior versions already handed off.** Edge Slide retains its earlier 15 numeric
oracles; Add Loop adds 20 measured desktop oracles.
Observed in Modo 17.1v1 on 2026-11-02 by desktop control (Codex), results read back from Modo (six decimals). REA-assisted: no. Earlier Edge Slide observations were scripted on 2026-11-02.

Covered: Add Loop midpoint/quarter/three-quarter placement, two drag lengths
and zooms, repeated clicks, triangle/pentagon/pole/boundary fixtures, Split,
option prerequisites and unequal-width fractional propagation.
See BEHAVIOR-ADD-LOOP.md and BEHAVIOR-EDGE-SLIDE.md.

Not covered: live held-drag preview, physical modifiers, full off-midpoint
Middle/Both Sides behavior, Caps/Gap with Split on, other placement modes,
curvature and undo.

G3: 3×3 quads in XY, X/Y=0…3; point index=row×4+column. Polygon
corners are [a,a+1,a+5,a+4]. G3-irregular uses columns X=0,0.5,2,3
and shifts the Y=3 row +0.5 in X. The additional variable-width fixture
moves point 10 to (2.5,2,0); oracles include every coordinate.

## Batch 2 additions

Observed in Modo 17.1v1 on 2026-11-02 by scripted probes (Codex), results read back from Modo by script (six decimals). REA-assisted: no. BEHAVIOR-BATCH-2.md adds 16 Edge Slide direction controls, preserving all prior oracle values and IDs. Positive direction is not explained by lower indices or winding/pick order alone; rotated/reflected results are pinned, general sign rule remains open. Vertex Slide and further Add Loop option gestures remain blocked by coordinate input after relaunch.


## Physical modifier additions

Four physical click cases observed on 2026-11-03 UTC (2026-11-02 local),
Jason at the mouse and Codex reading six-decimal results. The unmodified
control landed at 43.5%; Shift at 25.5%; Control at 50%; Command inserted
no loop and opened no popup. BEHAVIOR-PHYSICAL-MODIFIERS.md adds four
oracles (55 total), preserving all 51 prior oracle objects. Actual click
pixels were not captured, so no universal modifier law or exact same-pixel
comparison is claimed. Earlier physical-modifier blocked text is historical;
remaining options, contexts and drag-state coverage stay open.
