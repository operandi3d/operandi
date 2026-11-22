# Physical item ranges and second-polygon pick — v3 draft

Observed in Modo 17.1v1 on 2026-11-03 UTC (2026-11-02 local). Jason performs
physical input; Codex reads exact ordered selection, six-decimal geometry,
item channels and visibility, and inspects native UI. REA-assisted: no.

**SEL-PHY-01 — Shift-clicking down the item list includes hidden rows.**
Four consecutive Mesh rows A, B, C, D; B Visibility Off, other rows Default.
Starting with A alone selected, hold Shift and click the D row label.
The ordered result is [A,B,C,D], including hidden B. Visibility, transforms
and local geometry are unchanged. Oracle SEL-O-H6-forward-retry.

**SEL-PHY-02 — Reversing the range changes its recorded order.**
With the same four rows, start with D alone selected and Shift-click A.
The ordered result is [D,B,C,A], including still-hidden B. Geometry,
transforms and visibility remain unchanged. Oracle SEL-O-H6-reverse.
This is the measured sequence; it does not establish a general ordering
law for arbitrary hierarchies, filtering, reordering or collapsed groups.

All four names appear orange after either range. The starting row (A in
the forward trial, D in reverse) has a darker background; Properties Name
shows mixed. These observations do not independently establish which
item Modo treats as primary. Primary semantics remain an open question;
they must not be inferred solely from the returned order.

The earlier Item Pattern case selects a named group, including its hidden
member, as [B2Range1,B2Range2,B2Range3,B2Range4]. These physical trials add
positional-range evidence; they do not replace or reinterpret that case.
An initial attempt created an unexpected instance of A; the actual gesture
was unknown, so it was excluded, removed and the four-row fixture reset.

**SEL-PHY-03 — Shift+Command second-polygon picking selects a row.**
Use the README Grid4 in Front view with polygon 0 (bottom-left) selected,
snapping and symmetry off, no active Work Plane or modeling tool. Hold
Shift+Command and left-click the center of polygon 3 (bottom-right).
The ordered polygon selection becomes [0,1,2,3]. All 25 point positions and
16 quad corner lists remain unchanged. Native UI shows the four bottom
faces selected. Oracle SEL-O-H7-second-polygon contains the full mesh.
The actual physical click pixel was not recorded; grid calibration is
approximately 210.75 px/m. Other rows, direction reversals, non-row paths,
different topology and modifier changes during the gesture remain open.

Undo was not tested for these three physical actions.
