# Primitives: v2 draft questions

Answered here (exact layouts and counts in ORACLES.json): **SP-Q13 /
CYL-Q16** default UV layouts; **BOX-Q10** rounding counts; **SP-Q09 / SP-Q10**
sphere modes and levels; **CYL-Q02 / CYL-Q12** caps and axis; **CA-Q01**
odd-sided quad caps; **CA-Q03/Q04** zero and negative heights; **BOX-Q08**
negative size numerically; **EA-Q02** compensated UVs at zero/negative size
(data recorded).

Still open:

- PRIM-Q01 — Partly answered: level-zero cage half-side is approximately
  1.309182×Size for equal sizes 0.5/1/2 (PRIM-B3-01). Size is not raw
  cage half-side or corner radius; smoothed-limit meaning remains open.
- BOX-Q14 — Requested Radius 0.1/0.2 with Bulge 0.2 measured
  (PRIM-B3-02); general deformation rule and zero-Bulge controls remain open.
- PM-Q01–Q04, BOX-Q04/Q05/Q09, SP-Q01/Q04, CYL-Q01/Q03, EA-Q01: viewport
  drawing, handles, modifier clicks: need pointer input.
- PRIM-Q02: polygon types subdiv/psubdiv (stored type only; display is
  EX-Q06).
- PRIM-Q03: undo/redo (not drivable by script).

- PRIM-K-03 / CA-Q03/Q04 — Extended to Sides 5/12 and all four Bottom Cap types at Size Y 0/−0.75. Top Cap held Single. Last negative 12-side Radial case has a complete pre-crash readback and was not repeated; independent Top Cap variation remains open (PRIM-B3-03/04).
