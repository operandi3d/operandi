# Edge weights: build spec v2, 2026-11-05

Replaces v1 (`../BEHAVIOR-EDGE-WEIGHTS.md`) where they differ. Behaviour
only. Based on 82 new scripted measurements in Modo 17.1v1 (one cube, one
weighted edge at −300 % … +500 %, levels 1–3, plus two-edge and whole-face
cases, both smooth kinds), the 2026-11-03 measurements, and the user guide.
Tests: `ORACLES-EDGE-WEIGHTS-v2.json` (105 cases; weights as fractions,
1.0 = 100 %).

## What changed from v1

1. Negative weights matter (Jason's veto of v1's "negatives do nothing"),
   but **only on the Subdivision kind (Tab)**. Measured again: on
   Catmull-Clark (Shift+Tab) every negative value, down to −300 %, gives
   exactly the unweighted surface.
2. The weight's effect stops at ±100 %.
3. Modo's Subdivision kind does **not** behave like a crease. v1's EW-05 is
   withdrawn as the target; see EW2-04 for the interim rule.

## Rules

**EW2-01 — Range.** The Weight field accepts any value (Modo's own tool
shows −200 %). The effect is clamped: anything at or above 100 % acts as
100 %, anything at or below −100 % acts as −100 %. Measured on the
Subdivision kind at every level: +150, 200, 300 and 500 % match 100 %, and
−150, −200 and −300 % match −100 %. Store the typed value; clamp only when
building the surface.

**EW2-02 — Catmull-Clark: v1's EW-03 is confirmed.** Sharpness = 10 × weight,
semi-sharp crease rules. Weights from 75 % to 500 % all give the same surface
at levels 1–3, as expected, because the edge stays sharp for 7 or more levels.
Negative weights have no effect. No change for the build.

**EW2-03 — What Modo's Subdivision kind does (measured; exact rule
unknown).** On a unit cube with one weighted edge:

- **Positive:** the surface leans toward the cage edge. Not just a thin
  crease: the two faces beside the edge are pulled toward it too. At 100 %,
  level 1, the point on the edge's middle moves about half way (51 %) from the
  smooth surface toward the cage edge, and the centres of the two side faces
  move 0.11 toward the edge. At a fixed level the effect grows exactly as
  a·w + b·w² (faster than linear): 25 % gives about a fifth of the 100 %
  movement, 50 % about 43 %.
- **Negative:** the edge sinks in and the surface around it is pushed
  away from the cage. At −100 %, level 1, the edge's middle moves inward
  (0.2726 → 0.2188 on each axis), its two ends move inward along the edge
  (0.236 → 0.1875), and the centres of the two side faces move **away from**
  the edge and outward (by 0.08 and 0.03). The two faces at the ends of the
  edge (ones that touch it only at a corner) move slightly too. Negative is
  not the mirror image of positive.
- **100 % is not a fully hard edge** on this kind at levels 1–3, despite
  the guide's wording.

The exact formula is still open (SDV-Q02). Five model families were fitted
and none gets within 3 % of the cube's size, so no formula is given here
on purpose.

**EW2-04 — Interim rule for the Subdivision kind (Operandi choice; replace
when EW2-03 is solved).** Keep v1's EW-05 for positive weights, using the
weight clamped to [−1, 1]. For negative weights, run the same rule with the
blend fractions negative, so it extrapolates away from the crease: the edge
point moves away from the edge midpoint, and the patch controls on the edge
move away from their boundary-style values. Result: positive looks harder,
negative sinks the edge and softens it. That's the right direction, but it
won't match Modo point for point. Don't test the Subdivision kind against
Modo's oracles yet; give the rule its own snapshot tests.

**EW2-05 — Tool.** The panel's Weight field allows values beyond ±100 %
(EW2-01). The help text must stop saying negatives have no effect. Suggested:
"Positive pulls the surface toward the edge, negative pushes it away (Tab
surfaces only; Catmull-Clark ignores negatives). The effect stops at
±100 %." Modo's tool also has Additive (add to existing values instead of
replacing), Show Weight (numbers on the vertices) and Fine Steps (slower
drag). Additive is worth adding; the other two can wait.

**EW2-06 — Display (bug Jason reported, not a research item).** In
subdivision display, the selected or highlighted edge is drawn on the smooth
surface, not on the cage. Modo draws it on the surface. Already with the
build thread.

## Open

- EW2-03's exact rule (more measurements planned: flat grids give a cleaner
  signal than the cube).
- Whether Modo stores values beyond ±100 % or clamps them on entry (couldn't
  read the stored values back by script).
- Weights where two or more weighted edges meet (measured in the oracle
  file: `corner` and `face` cases, ±100 %; no rule yet).
