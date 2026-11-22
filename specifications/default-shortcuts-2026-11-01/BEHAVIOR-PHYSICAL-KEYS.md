# Physical macOS shortcut checks — v2 draft additions

Modo 17.1v1, 2026-11-03 UTC (2026-11-02 local). Jason presses the physical
keys; Codex verifies native tool UI and reads exact mesh/selection output.
Current active configuration, no factory reset. REA-assisted: no. Each
trial starts from an identity unit cube (coordinates ±0.5), Polygon mode,
only face 1 at Z=+0.5 selected, no active tool. Modo is focused through its
title bar and the pointer placed over the 3D viewport cube. No click or
drag is requested after the keypress. These are measured current-context
bindings, not a claim about every mode or keyboard configuration.

| Requirement | Physical key | Observed result | Oracle |
|---|---|---|---|
| KEY-PHY-01 | Shift+C | Polygon Slice activated; Slice panel/status confirmed | KEY-O-H2-shift-c |
| KEY-PHY-02 | Option+C | Loop Slice activated, with a live one-cut preview | KEY-O-H2-option-c |
| KEY-PHY-03 | Control+E | Bend activated; Angle 0, Spine (0,1,0) | KEY-O-H2-control-e |
| KEY-PHY-04 | Command+E | No visible tool activation or mesh/selection change observed | KEY-O-H2-command-e |

**KEY-PHY-01 — Physical Shift+C activates Polygon Slice in this context.**
All eight points, six quads and the selected face are unchanged after
activation. No cut is drawn. This physical result differs from the earlier
synthetic-input Channel Haul diagnostic; that diagnostic is retained and
does not define the physical shortcut.

**KEY-PHY-02 — Physical Option+C activates Loop Slice; the active preview already contains the measured cut.**
Retained Edit Move, Mode Free, Count 1, Inset 0, Select New Polygons and
Draw Sliced Edges on. Active readback has 12 points / 10 quads instead of
8/6: four new points at X=0, Y/Z=±0.5 form one ring. Eight new faces
(indices 2…9), new vertices 8…11 and the ring edges are selected in their
respective component readbacks. Exact corner order is in the oracle.
The tool remains active, so this does not distinguish committed topology
from live preview. No additional pointer click was reported. Other retained
settings and contexts could produce a different preview.

**KEY-PHY-03 — Physical Control+E activates Bend in this context.**
The Bend panel and native status confirm the name; displayed Angle is zero,
Spine (0,1,0). Geometry and component selection stay unchanged. The preset
also displays Automatic in the toolbar and Auto center/axis in its status;
this is an observed preset display, not a claim about a separately selected
named Action Center.

**KEY-PHY-04 — No visible tool activation is observed after physical Command+E in this context.**
Jason reports no activation, the native UI exposes no new tool panel/status,
and none of the queried tools is on. Exact mesh and component selection are
unchanged. This does not establish that the key is unbound, that no other
command ran, or that every context behaves the same. Current Input Editor
assignment for this pair was not revisited.

The physical Control/Command E trials have different observed results.
Physical Shift/Option C also work as distinct modified keys. Earlier injected
modifier anomalies remain input-delivery diagnostics; their underlying cause
is still open. No undo/redo or exhaustive key inventory is added here.
