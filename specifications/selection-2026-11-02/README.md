# Selection (TOOL_GAPS section 2)

**Status: v3 draft (partial); v1 already handed off.** Observed in Modo 17.1v1 on 2026-11-02 through scripted
probes (commands, not pointer picks). REA-assisted: no.

## Covered

- `BEHAVIOR-SELECT-BETWEEN.md`: Select Between, including ties (SEL-Q-M1),
  rectangles and more than two picks.
- `BEHAVIOR-LOOPS.md`: Loop and Previous/Next Loop (SQ-009 part).
- `BEHAVIOR-ITEMS.md`: parenting with and without keeping the world
  position, and component edits on several selected mesh items (SQ-008 part).
- `ORACLES.json`: the measured cases.

## Not covered (need pointer input or have no command)

Select Through, picking on the subdivision cage, lasso visibility (SEL-U03),
selecting a row by modifier-clicking a second polygon (SEL-Q-M4), Polygon
Selection Fill (SEL-Q-M2: no scriptable command was found), item ranges and
reordering in the item list. See QUESTIONS.md.

## Fixtures

- **Ring8**: 8 quads around the Y axis. Points 0–7 at y = 0 and 8–15 at
  y = 1, point i at angle 45°·i in the XZ plane, radius 1. Polygon i is
  `[i, i+1, i+9, i+8]` (indices mod 8 within each row).
- **Grid4**: 4×4 quads in XY, x and y 0…4. Point = row × 5 + column;
  polygon = row × 4 + column, polygon `[a, a+1, a+6, a+5]` with
  a = row × 5 + column.
- **Box**: points (±1, ±1, ±1) in the order (-1,-1,-1), (1,-1,-1), (1,-1,1),
  (-1,-1,1), (-1,1,-1), (1,1,-1), (1,1,1), (-1,1,1); faces 0 bottom
  `[0,1,2,3]`, 1 top `[7,6,5,4]`, 2 `[0,4,5,1]`, 3 `[1,5,6,2]`,
  4 `[2,6,7,3]`, 5 `[3,7,4,0]`.

## Batch 2 additions

Observed in Modo 17.1v1 on 2026-11-02 by desktop control (Codex) and scripted probes; results read back from Modo by script (six decimals). REA-assisted: no. BEHAVIOR-BATCH-2.md adds 40 oracles, preserving prior IDs and values. Covered: interior loop directions, more three-pick cases, polygon-corner tie discriminator, rectangle order, plain/menu Parent, hidden item by name-pattern, selection Undo control. Fill, positional item ranges and confirmed UI primary identity remain open. Older Not covered text describes v1; current dispositions are in QUESTIONS.md.


## Physical selection additions

BEHAVIOR-PHYSICAL-SELECTION.md adds three physical observations in Modo
17.1v1 on 2026-11-03 UTC: forward and reverse item-list Shift ranges include
hidden B, yielding [A,B,C,D] and [D,B,C,A]; Shift+Command second-polygon
picking selects Grid4 row [0,1,2,3]. There are 72 current oracles; all 69
previous objects are unchanged. Earlier Not covered text describes prior
passes. General hierarchy/reorder behavior, confirmed primary semantics,
broader polygon-pick cases and the remaining selection gaps stay open.
