# Falloff — v2 draft additions

Observed in Modo 17.1v1 on 2026-11-02 by scripted probes and live native
Tool Properties/Edit-menu actions (Codex); six-decimal scripted readback.
REA-assisted: no. Symmetry off except where stated; no snapping; fresh input
each time. Numeric point tolerance 1e-5; counts, selection and topology exact.

**FAL-B2-01 — The tested Extrude changes topology at exactly zero terminal weight.**
Two-quad strip: X=−1,0,1 at Y=0 and 1, Z=0; both polygons selected.
Extrude Auto, one segment, Shift (0,0,1). Linear shape with the following
Start/End along X; other coordinates zero:

| Weight at X=1 | Start X | End X | Result points / polygons |
|---|---|---|---|
| 1 | 1 | 2 | 12 / 10 |
| 0.5 | −1 | 3 | 12 / 10 |
| 0.1 | −1 | 1.2222222222222223 | 12 / 10 |
| 0.01 | −1 | 1.0202020202020203 | 12 / 10 |
| 0 | −1 | 1 | 10 / 7 |

For positive terminal weight both quads receive raised copies, with Z equal
to the original corner's weight. At zero, X=1 points are not duplicated,
the right input quad stays at Z=0, and only the left quad is raised with
the partial wall topology recorded in the oracle. Thus 0.01 is sufficient
to retain the full topology in this setup; the zero case is not merely a
zero-length version of that topology. Selection and every polygon list:
FAL-O-F2-01-w1/w0.5/w0.1/w0.01/w0. This resolves the sampled zero-versus-
small-positive question in FAL-T-04; arbitrary mixtures remain open.

**FAL-B2-02 — Weighted Rotate blends point positions, rather than scaling the rotation angle, in the tested strip.**
Five quads along X=0…5, two rows Y=0,1; all 12 points selected, Origin.
Linear Start (0,0.5,0), End (5,0.5,0), so w=1−X/5. Enter Rotate Z 90°
through live Tool Properties (the scripted angle write was disabled).
Measured result is p′=p+w(R90(p)−p), within six-decimal readback. For
(1,0,0), w=0.8: result (0.2,0.8,0), rather than rotation through 72°.
For (2,1,0), w=0.6: (0.2,1.6,0). Geometry remains 12/5 and all points
selected. Full coordinates: FAL-O-F2-03-Rotate. The displayed falloff line
followed the live deformation afterward; the stated line is the initial UI
reading used to derive weights.

**FAL-B2-03 — Uniform 200% Scale uses factor 1+w on each tested point.**
Same strip and weights (initial line Y=0 also gives the same X projection),
Origin, numeric Scale X/Y/Z=2. At X=1 the factor is 1.8, at X=2 it is
1.6, down to 1 at X=5. Example (2,1,0) → (3.2,1.6,0). Counts 12/5,
all points selected. FAL-O-F2-03-Scale.

**FAL-B2-04 — X symmetry samples the tested falloff at the positive-side position, including for a selected negative-side point alone.**
Isolated points (−1,0,0) and (1,0,0), X symmetry, Move Y 1. Linear
Start 0, End (2,0,0): ordinary own-position weights would be 1 and 0.5,
but both selected points move by 0.5 m. With only the negative point
selected, it still moves by 0.5 m and the positive point stays unchanged.
With only the positive point selected, only it moves by 0.5 m. Reverse
End to (−2,0,0), both selected: both move 1 m, the weight at +1. This
discriminates positive-side sampling from taking the smaller weight.
FAL-O-F2-04, FAL-O-F2-04-negative-only/positive-only/both-reversed.
Other axes, offsets and field types remain open.

**FAL-B2-05 — Zero Z Size makes the tested Radial weight independent of Z.**
Center zero, Size (1,1,0), Linear shape; all isolated test points selected,
Move Y 1. Points (0,0,Z), Z=0,0.01,−0.01,1 all gain Y=1. Points
(0.5,0,0) and (0.5,0,0.01) gain 0.5. Point (0,0.5,0) gains 0.5;
(0.5,0.5,0) gains 0.292893. Thus the tested zero axis is ignored, rather
than restricting influence to its center plane or assigning zero weight.
FAL-O-F2-05 extends FAL-R-01 to this zero-axis case. Multiple zero axes
and negative sizes remain open.

**FAL-B2-06 — Custom In/Out labels correspond to the measured curve parameters in the stated order.**
With Custom selected, set the UI's In to 0.2 and Out to 0.8; script readback
confirms distinct stored values in that order (raw record F2-07). A fresh
Linear falloff with those values, Start 0, End (1,0,0), Move Y 1 gives
weights [1,0.646875,0.425,0.240625,0] at X=[0,0.25,0.5,0.75,1].
These match the earlier Custom formula using the UI's In=0.2 and Out=0.8,
confirming its previously inferred label assignment for this sample.
FAL-O-F2-07 and FAL-O-F2-07-curve. Other custom values remain governed
by the original formula's stated inference limits.

**FAL-B2-07 — Both Use World Transforms settings sample the translated mesh at world X in these numeric trials.**
Local strip X=0,1,2, Y=0,1; mesh item translated to (3,0,0); all six
vertices selected, Move Y 1. Linear Start (3,0,0), End (5,0,0) gives
displacements 1,0.5,0 at the three columns, with Use World Transforms off
and on. Start 0, End (2,0,0) gives no displacement with either setting.
The flag was read back as 0/1 as requested; its lack of difference here is
limited to these translated-item numeric cases, not a universal no-effect
rule. Counts 6/2, same selection. FAL-O-F2-08-world-off/on and
FAL-O-F2-08-local-off/on. Rotation/scale of the item remain open.

**FAL-B2-08 — The sampled falloff Move undoes in one Edit-menu step.**
Six-point strip X=0…2, Y=0,1, all vertices selected, Linear Start 0,
End (2,0,0), Move Y 1. One Edit > Undo restores every original coordinate
and polygon list. FAL-O-F2-09-Move includes applied and restored states.
No scripted-readback Redo claim follows from this case.

Smooth Shift crashed during UI discovery before its planned falloff fixture;
that case was skipped. Thicken's Polygon-tab button and Selection/Soft
Selection/Element handle drags could not be reached after relaunch because
coordinate input reported no available window. They establish no behavior
requirements here; native fields and menu actions continued to work.
