# Add Loop physical modifier clicks — v4 draft additions

Modo 17.1v1, 2026-11-03 UTC (2026-11-02 local). Jason performs physical
clicks and reports any deviation; Codex reads six-decimal mesh, selection
and tool values. REA-assisted: no. Fresh identity G3 before each trial,
Edge mode, nothing selected, Add Loop active. Middle, Both Sides, Split,
Caps, Absolute Distance and Preserve Curvature off, Gap 0, Position 0,
Show Position on, snapping off. Front orthographic, approximately 281 px/m
(3 m spans about 843 px, endpoint reading ±1 px). Coordinates below are
local/world metres because the item has identity channels.

Click target: bottom edge 1–2, (1,0,0)→(2,0,0), requested near its quarter
point (1.25,0,0), approximately window pixel (1133,1122) in a 2476×1364
window. Actual human click pixels were not recorded. Modifier, when present,
is held before mouse-down until after mouse-up; no drag requested.

| Requirement | Physical trial | Measured Position | Output points / quads | Cut |
|---|---|---:|---:|---|
| AL-PHY-01 | Unmodified control | 43.5% | 20 / 12 | one line at X=1.435 |
| AL-PHY-02 | Shift-left-click | 25.5% | 20 / 12 | one line at X=1.255 |
| AL-PHY-03 | Control-left-click | 50% | 20 / 12 | one line at X=1.5 |
| AL-PHY-04 | Command-left-click | remains 0% | 16 / 9 | no inserted loop; no popup reported |

**AL-PHY-01 — The unmodified control uses its actual 43.5% placement.**
Jason reports missing the requested quarter point and landing near the
middle. Exact readback gives Position 43.5%, not 25% or 50%. Four new
points at (1.435,Y,0), Y=0,1,2,3, split the three crossed quads. All component
selections remain empty. AL-O-H4-unmodified. This is a valid off-center
control observation; it is not an exact same-pixel comparison with the
later clicks.

**AL-PHY-02 — The measured Shift-click inserts a single off-center loop.**
Readback gives Position 25.5%, four points at X=1.255 and the same 20/12
single-loop topology. Middle, Both Sides and Split are off after release;
component selection is empty. AL-O-H4-shift. This sample did not force a
midpoint. No exact placement equality to an unmodified click is asserted,
because the actual input pixels differed and were not captured.

**AL-PHY-03 — The measured Control-click gives a midpoint loop.**
Requested off-center click gives Position exactly 50%, four new X=1.5
points, counts 20/12, component selections empty. Middle, Both Sides and
Split are off after release. AL-O-H4-control. This records the midpoint
result; no transient held-modifier checkbox state was sampled, and one
manual input does not establish a universal centering law.

**AL-PHY-04 — The measured Command-click inserts no loop and opens no popup.**
The 16 points, nine quads and empty component selections remain identical
to input. Add Loop stays active, Position stays zero and no edge is picked.
Jason explicitly reports that no popup menu appeared. Zero Position is a
retained value, not evidence of a cut at an endpoint. AL-O-H4-command.
This active-tool context differs from the earlier selected-cube popup
contexts. Broader Command-click cancellation/selection behavior is open.

Every oracle retains full output coordinates and polygon orders. The four
cases establish these physical results only: exact hit-point matching,
press/release changes during a drag, other views and additional placements
remain open. Original independent Add Loop settings and snapping were
restored afterward; derived distances and picked-edge references were not
used as preference checks. No screenshot or internal command is needed
to interpret these observations.
