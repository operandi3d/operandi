# Symmetry — v2 draft additions

Observed 2026-11-02 by scripted probes and desktop pointer/menu
actions in Modo 17.1v1. Six-decimal readback; REA-assisted: no. F1 is defined
in README. Fresh input per operation, no falloff or snapping, Origin center
for numeric transforms. Geometric tolerance in oracles is 1e-5; selection
membership/order, counts and topology are exact, including micron tests.

**SYM-B2-01 — Select Symmetrical retains pick order and appends only missing counterparts.**
F1, X, Topology off. Picks [9,8,6] → [9,8,6,5]; [6,9,8] → [6,9,8,5];
[8,6,9] → [8,6,9,5]. Extra discriminators: [8,9] → [8,9,6,5];
[9,6] → [9,6,8,5]; [6,9] → [6,9,8,5]. In these two-addition cases the
new indices descend independently of pick order. This does not prove a
universal index-sort rule. Geometry stays 10 points / 4 polygons.
Oracles SYM-O-S2-01-1 through SYM-O-S2-01-6.

**SYM-B2-02 — Geometric matching tolerance changes under translation even when bounding extents stay fixed.**
Move point 5 toward +X by the stated error, select point 9, then Select
Symmetrical (Topology off). A match appends point 5; failure leaves [9].

| Variant | Matching offsets (µm) | Nonmatching offsets (µm) |
|---|---|---|
| F1 stretched in Y, Y=0…10 | 5, 8, 20, 25, 26, 27, 28, 29 | 30, 35, 40, 50 |
| Top row at Z=10, Y=0…1 | 5, 8, 20, 25 | 30, 35, 40, 50 |
| Y and Z both 0…10 | 5, 8, 20, 25, 26, 27, 28, 29 | 30, 35, 40, 50 |
| Same Y extent, centered at Y=0 (−5…5) | 5, 8, 14 | 15, 16, 20, 26…29, 50 |
| Same Y extent, translated to Y=100…110 | 26…29, 50, 100, 200, 300 | 330, 350, 500 |
| F1 with top row Z=1 | 5 | 8, 20, 50 |

The Y=0…10 and Y=−5…5 inputs have the same X/Y/Z extents and diagonal,
yet 20 µm matches only the former. Thus neither bounding-box diagonal,
largest side nor X extent alone determines tolerance. The samples fit a
threshold near 3×10⁻⁶ times the largest absolute coordinate, with equality
not reliably accepted; this formula is an inference, not a complete law.
Every before/after and selection is in SYM-O-S2-02-<variant>-<error>um.
This supersedes the general size-only interpretation in SYM-SEL-05 while
retaining its original measured examples. *(open: SYM-Q04, exact formula)*

**SYM-B2-03 — A selected plane point remains on the plane under the tested Rotate and Scale.**
F1 points [7,9], X symmetry. Fresh Rotate Z 30° gives point 7=(0,0.866025,0);
fresh Scale X 1.5 leaves it (0,1,0). It never gains an X coordinate.
Counts 10/4 and selection [7,9] remain. SYM-O-S2-03-Rotate/Scale.

**SYM-B2-04 — The small Ellipsoid and Cone mirrored copies keep the first corner when reversing winding.**
Ellipsoid center (1,0,0), Size (0.5,0.5,0.5), Sides 8, Segments 4,
Y axis: Off 26 points / 32 polygons; X 52/64. Cone same center, Size
(0.5,0.75,0.5), Sides 8, Segments 1, single caps: Off 9/9; X 18/18.
In each, originals come first; point N+i mirrors original i across X=0.
Each mirrored polygon [a,b,…,z] becomes [a′,z′,…,b′]. Every polygon was
compared, not just counts. Selection empty. SYM-O-S2-04-off/X and
SYM-O-S2-05-off/X.

**SYM-B2-05 — Cube doubling depends on whether its center lies on the plane.**
Unit Cube, one segment per axis, empty mesh, X symmetry. Center (0,0,0)
gives a single 8-point/6-polygon cube, identical to Off. Center (0.25,0,0)
gives 16/12: two overlapping copies centered at X=±0.25, not a merged
surface. Selection empty. SYM-O-S2-06-center0-off/X and
SYM-O-S2-06-center025-off/X. This is an exception to SYM-PR-01's
general doubling statement; its center-X=1 example remains valid.

**SYM-B2-06 — Symmetrize pairs the displaced strip endpoints but leaves the tested unpaired point unchanged.**
F1: add δ to the X coordinates of both points 5 and 9, δ=0.01,0.05,0.2 m;
add isolated point 10=(1.5,0.5,0). Positive→Negative, no selection.
Point 9 stays (2+δ,1,0); point 5 becomes (−2−δ,1,0). Plane point 7
and isolated point 10 stay fixed. No points/polygons added or removed:
11/4, selection empty. SYM-O-S2-07-0.01/0.05/0.2. This does not define
all pairing ambiguities or maximum pairing distance. *(open: SYM-Q09)*

**SYM-B2-07 — Topology finds the displaced counterpart on the symmetric strip, but not after the tested asymmetric cut.**
Move F1 point 5 from X=−2 to −1.7, select [9]. Position matching leaves
[9]; Topology on gives [9,5]. Add a cut at X=1.5 through the positive
outer quad, adding two points and a fifth polygon: both modes leave [9].
Exact topology in SYM-O-S2-08-symmetric-position/topology and
SYM-O-S2-08-cut-position/topology.

**SYM-B2-08 — The moved Work Plane changes the tested counterpart and numeric Move result.**
F1, Work Plane center (1,0,0), no rotation; X symmetry, Work Plane on,
Offset zero. Select [9] then Select Symmetrical → [9,7], matching about
X=1. Fresh [9], numeric Move (0.25,0.1,0) gives point 9=(2.25,1.1,0),
others unchanged. Record the positive X result as measured; the numeric
zero-work-plane transform rule is not sufficient to predict this case.
SYM-O-S2-10-workplane-select/Move. General displaced-plane transform
composition remains open.

**SYM-B2-09 — The tested Arbitrary vector uses the diagonal mirror plane.**
Points [(1,0,0),(0,−1,0),(−1,0,0),(0,1,0)], one quad [0,3,2,1].
Arbitrary Axis (1,1,0), Offset zero, Work Plane off: Select Symmetrical
from [0] adds [1]. Fresh [0], Move (0.25,0.1,0) gives point 0=(0.9,−0.25,0),
counterpart unchanged. Counts 4/1. SYM-O-S2-10-arbitrary-select/Move.

**SYM-B2-10 — A pointer polygon pick adds its counterpart; a one-sided handle drag leaves the unselected counterpart unchanged.**
F1, Polygon mode, X, Topology off. Front 231 px/m. Click (879,588) in
polygon 3 → ordered selection [3,0]. Restore only polygon 3 by script,
activate Move, then X-arrow drag (974,588)→(1074,588), 100 px
(0.432900 m projected). Points 3,4,8,9 gain +0.435000 m in X;
polygon 0's points do not move; selection stays [3]. Counts 10/4.
The positive handle direction differs from entering a positive numeric
offset in earlier Origin-center probes. SYM-O-S2-11 records both stages.

**SYM-B2-11 — The tested mirrored Move and symmetric Cube each undo in one menu step.**
F1 [5,9], X, Move (0.25,0.1,0): one Edit > Undo restores all input points
and selection [5,9]. Empty mesh, X, unit Cube center (1,0,0): one Undo
restores 0 points / 0 polygons. SYM-O-S2-13-Move/Cube. A separate Cube
control does immediate Edit > Undo then Edit > Redo without intermediate
scripted readback; the final mesh exactly matches the applied 16/12 result
(SYM-O-S2-13-control-Cube). Scripted readback made Redo unavailable
in the Move trial, so no general inability to redo is inferred. Historical
SYM-TR-07's unobserved status is superseded for these setups.

**SYM-B2-12 — Both selected outer polygons are edited once each by the tested topology tools.**
F1 polygons [0,3], X. Bevel Inset 0.1, Shift 0.2 m → 18 points / 12
polygons, new corners X=±1.1/±1.9, Y=0.1/0.9, Z=0.2; selection [3,0].
Fresh Extrude Shift Z 1, one segment → 18/12, raised copies at Z=1,
selection [10,11]. SYM-O-S2-12-both-Bevel/Extrude.

**SYM-B2-13 — A single straddling polygon is edited as one polygon in the tested cases.**
Replace central quads 1 and 2 by [1,3,8,6], leaving 10 points / 3 polygons;
select only that polygon, X. Bevel 0.1/0.2 → 14/7, new corners
(±0.9,0.1/0.9,0.2), selection [1]. Fresh Extrude Z 1 → 14/7,
one four-corner raised face, selection [6]. SYM-O-S2-12-straddling-Bevel/Extrude.

UV symmetry remains inconclusive: UV Active, UV Axis U, UV Offset 0.5
were configured on a split unit plane, but numeric UV Move left UVs
unchanged with symmetry both off and on. The UV transform target was not
established, so there is no UV-symmetry acceptance rule from that attempt.
