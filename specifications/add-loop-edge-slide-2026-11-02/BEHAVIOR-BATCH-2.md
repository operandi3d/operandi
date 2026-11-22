# Edge Slide direction — v3 draft additions

Observed numerically in Modo 17.1v1 on 2026-11-02 (Codex), six-decimal
scripted mesh readback. REA-assisted: no. Fresh G3, edge loop 1–5–9–13,
Radial mode, Percent interpolation 0.5, Merge/Loop on, Stop/Duplicate off,
other options reset. No symmetry, snapping or falloff. View remained the
post-relaunch Perspective view for representation controls. Later Front/Back
controls used the correct view command; earlier rejected attempts
are excluded.

**ES-B2-01 — Positive slide direction survives the tested index, corner and selection-order changes.**
On G3 the selected loop moves from X=1 to X=0.5. The same spatial result
occurs after reversing polygon-list order; reversing every polygon's winding
by whole-list or keep-first reversal; cyclically shifting every corner list
by two; reversing all point indices with topology and selection remapped;
reversing edge-pick order; and reversing endpoint argument order. A second
renumbering control sorts each edge's endpoints after remapping and still
gives X=0.5. Thus neither lower point index nor these pick/corner orders
alone explain the positive side. All counts stay 16/9 and selected edges
are retained. Oracles ES-O-ES2-01-baseline, reverse-polygon-list,
reverse-winding, keep-first-reverse, cyclic2, reverse-point-indices,
reverse-pick-order, reverse-endpoints, reverse-points-sorted-endpoints.

**ES-B2-02 — Rotated and reflected geometry give the following measured positive directions.**
Transform only G3's point coordinates, retaining its topology and chosen loop:

| Variant | A selected boundary point before → after |
|---|---|
| Rotate Z 90° | (0,1,0) → (0,0.5,0) |
| Rotate Z 135° | (−0.707107,0.707107,0) → (−0.353553,0.353553,0) |
| Rotate Z 180° | (−1,0,0) → (−0.5,0,0) |
| Rotate Y 90° | (0,0,−1) → (0,0,−0.5) |
| Reflect X by X′=3−X | (2,0,0) → (1.5,0,0) |

Every loop point follows its corresponding neighboring edge by half its
length. Counts/selection unchanged. Oracles ES-O-ES2-01-rotate-Z90/Z135/
Z180/Y90 and reflect-X. In particular, the X-reflected case moves toward
the opposite topological neighbor from the reflected baseline result; a
rule that simply carries the original neighbor through every reflection
is insufficient. These cases also disprove an always-negative-world-axis
rule. They constrain direction but do not yet establish the general
handle/view basis or tie-breaking rule. *(open: ES-Q01)*

Vertex Slide and off-midpoint Add Loop Middle/Both Sides/Caps/Gap cases
remain unobserved in this batch: the desktop coordinate route reported
no available window after relaunch. Existing Add Loop requirements and
oracles retain their meanings; no modifier cases were attempted.

**ES-B2-03 — Front and Back views give the same positive displacement for the baseline numeric loop.**
Fresh G3 in each view, same loop and reset Radial/Percent settings: Percent
0.5 moves the loop from X=1 to X=0.5 in both views, retaining 16/9 and the
selected edges. ES-O-ES2-02-view-Front/Back. This rules out a simple screen-
left choice for these opposed views; it does not settle the general sign rule.
