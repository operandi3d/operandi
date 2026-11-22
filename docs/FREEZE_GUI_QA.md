# Freeze v2: GUI check (curves and the Freeze dialog)

Spec: `specifications/freeze-2026-11-10` (FRZ-01..10). Use the QA app with
its own data folder; don't open Jason's scenes.

1. New scene, Curve tab: draw a Bezier with two anchors (a single S-shaped
   span). Edit tab, **Freeze...**. The dialog shows Freeze Curves as
   (Faces), Tessellation and Subdivide Curves with Fixed Spans active;
   Curve Refinement Angle greyed while Fixed Spans is on, with a "Turn off
   Subdivide Curves with Fixed Spans first" tooltip; Make Holes, Drill
   Axis, Freeze Displacement, Freeze Deformation, Save Deformation to Morph
   and Morph Map Name greyed with "Not available yet".
2. OK: the curve becomes one polygon lying exactly on the drawn curve (no
   visible change in shape). Status "Frozen: 1 polygons". Cmd+Z brings the
   curve back with its handles in one step.
3. Freeze..., Freeze Curves as **2 Point Polygons**, OK: a chain of 36 line
   segments per span. Undo. **Polylines**, OK: one straight polyline.
4. Undo. Freeze..., untick Fixed Spans: Curve Refinement Angle enables.
   Set 20°, OK: visibly fewer segments, tighter bends get more. Undo, 2°,
   OK: many more.
5. Mixed: in the same mesh add a cube and Tab it to a subdivision surface.
   Item mode, Freeze (no dialog): cube and curve both freeze; one Cmd+Z
   restores both. Freeze... with Tessellation off: nothing changes, curve
   included, and the status line says "Nothing to freeze".
5a. Two curves, Polygon mode, select one, Freeze: only that one freezes;
   nothing is selected afterwards. Vertex mode with a point selected:
   "Nothing to freeze", the point stays selected.
5b. A Bezier frozen as Faces: its two anchors are corners of the new
   polygon (select them in Vertex mode); the handles are gone.
6. A mesh with only plain polygons and a straight polyline: Freeze says
   "Nothing to freeze: the mesh has no curves or subdivision surfaces" and
   adds no undo step.
7. Command line: `poly.freeze face:polylines`, `poly.freeze face:2
   fixed:false cangle:10deg` freeze without the dialog; `poly.freeze` alone
   opens it; `poly.freeze disp:false` notes "disp accepted and ignored".
