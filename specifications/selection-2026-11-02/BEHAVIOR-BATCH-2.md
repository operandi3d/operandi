# Selection — v2 draft additions

Observed in Modo 17.1v1 on 2026-11-02, scripted probes plus native menu and
dialog actions (Codex), six-decimal scripted readback. REA-assisted: no.
Grid4 and Ring8 are defined in README. Geometry unchanged unless a parenting
transform is specified. Selection membership and order are exact.

**SEL-B2-01 — Parallel-loop direction depends on selection mode in the recorded interior-loop cases.**
Grid4, fresh start per direction. Edge start is [(2,7),(7,12),(12,17),(17,22)],
X=2. Polygon start [2,6,10,14], column centered at X=2.5. Point start
[2,7,12,17,22], X=2. Each command replaces the selection, twice in succession:

| Mode / command | First result | Second result |
|---|---|---|
| Edge Next | X=1 loop | Entire 16-edge border |
| Edge Previous | X=1 loop | Entire 16-edge border |
| Polygon Next | [3,7,11,15] | [14,10,6,2] |
| Polygon Previous | [1,5,9,13] | [0,4,8,12] |
| Point Next | [3,8,13,18,23] | Entire 16-point border |
| Point Previous | [1,6,11,16,21] | Entire 16-point border |

Both edge directions taking the same first neighbor is the measured result
for the stated ordered edge selection, not a universal direction rule.
All border orders and topology: SEL-O-L2-01-Edge/Polygon/Vertex-next/prev.

**SEL-B2-02 — The two additional three-pick cases connect the final pair.**
Grid4 polygons, Select Between: [0,15,3] → [0,15,11,7,3]. Picks
[3,0,15] → [3,0,15,4,8,12,1,5,9,13,2,6,10,14,7,11], the full
rectangle between 0 and 15 with pick 3 retained first. This supports the
last-pair interpretation in SEL-BT-03 for these orders. SEL-O-L2-02-1/2.

**SEL-B2-03 — Ring8 tie routes depend on polygon corner-list representation, not just polygon indices or winding sign.**
Original polygon i=[i,i+1,i+9,i+8] (row indices wrap). Reversing the
whole list preserves all nine previously measured routes. For polygon
picks 0→4, the result remains [0,7,6,5,4]. Reversing while keeping the
first corner gives [i,i+8,i+9,i+1]; the polygon route flips to [0,1,2,3,4].
Rotating the original list by two corners, with winding unchanged, also
flips it. Thus a universal lower-index tie rule is disproved.

| Polygon picks | Full reversal | Keep-first reversal or cyclic shift by two |
|---|---|---|
| 0,4 | [0,7,6,5,4] | [0,1,2,3,4] |
| 4,0 | [4,3,2,1,0] | [4,5,6,7,0] |
| 1,5 | [1,0,7,6,5] | [1,2,3,4,5] |
| 5,1 | [5,4,3,2,1] | [5,6,7,0,1] |
| 2,6 | [2,1,0,7,6] | [2,3,4,5,6] |
| 3,7 | [3,2,1,0,7] | [3,4,5,6,7] |

In these representations, the chosen route leaves through the later of
the two traversable side edges in the first polygon's ordered boundary.
That describes these measured rings; extension to other topology remains
open. Point routes are unchanged under both reversals: 0,4→[0,1,2,3,4],
4,0→[4,3,2,1,0], 2,6→[2,3,4,5,6]. Oracles
SEL-O-L2-03-full-reverse-1…9, keep-first-reverse-1…9, cyclic2-1…6.
This supersedes SEL-BT-05's general lower-index interpretation, retaining
its original examples and the one-route rather than union result.

**SEL-B2-04 — The rectangle's added-element order changes with the picked corners.**
Same 3×3 block {0,1,2,4,5,6,8,9,10}, Grid4 Select Between:

| Picks | Added elements in returned order (picks excluded) |
|---|---|
| [0,10] | [4,8,1,5,9,2,6] |
| [10,0] | [9,8,6,5,4,2,1] |
| [2,8] | [1,0,6,5,4,10,9] |
| [8,2] | [9,10,4,5,6,0,1] |

The picks stay first. The 0→10 case traverses columns; the other three
traverse rows from their starting corner. None is a global ascending-index
order. A general traversal-axis rule beyond these fixtures is still open.
SEL-O-L2-04-1…4.

**SEL-B2-05 — Plain Parent and the UI Parent menu variant keep the child's local transform in this setup.**
Unparented Parent mesh: position (1,0,0), rotation (0,90,0)°, scale one.
Unparented Child: position (0,0,2), zero rotation, scale one. Plain
parenting with Keep Position omitted leaves Child local position (0,0,2),
rotation zero; evaluated world position becomes (3,0,0). The UI route
Item > Hierarchy > Parent, after selecting Child then Parent, gives the
same result. Parent channels stay unchanged. The menu separately lists
Parent in Place. Exact channels: SEL-O-L2-05-plain/menu. Item-list drag
and context-menu defaults were not reached, so these do not establish
their default variant. World values were read after an idle boundary;
immediate matrix readback after linking was stale.

**SEL-B2-06 — Item Pattern selects the matching named group including a hidden item.**
Four mesh items B2Range1…4, second item's Visibility Off, others Default.
Items mode; B2Range1 initially selected. Select > Item Pattern, Pattern
B2Range*, Mode Set, confirm without modifiers. Ordered selected items:
[B2Range1,B2Range2,B2Range3,B2Range4], including the hidden item.
SEL-O-L2-06-pattern. This is a name-pattern route, not a claim about
positional range selection. A primary item could not be confirmed from
the mixed Properties panel; it remains open.

**SEL-B2-07 — Native polygon Invert is undone in one menu step in the control without an intervening scripted read.**
Grid4 moved to Z=0.25 as a known geometry marker, polygon 0 selected.
Select > Invert selects polygons 1…15 without changing points. In a fresh
matching control, perform Invert then immediately Edit > Undo: polygon 0
is restored and every point remains at Z=0.25. Thus Undo reverses the
selection change, rather than the preceding geometry marker. Oracle
SEL-O-L2-08-control. A separate readback-interleaved trial left the inverted
selection through two menu Undos; that instrumentation-dependent trial is
excluded. Undo counts with a scripted readback between each native action are
not reliable in this route.

Polygon Selection Fill was not exposed in the inspected Select menu, and
the custom Select-tab route remained inaccessible to coordinate input.
No Fill behavior is asserted for either requested region.
