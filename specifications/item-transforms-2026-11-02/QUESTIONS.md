# Item transforms: questions

- **SQ-004 — Partly:** typed Move, Gimbal-on X rotation with ZXY order,
  handle examples and parented local Move measured. Control+Shift+left drag with Duplicate Mode Instance
  creates a linked instance and moves it, preserving the source (IT-PHY-04).
  Other duplicate modes and action-center combinations remain open.
- **SQ-M11-005 — Answered for the tested configuration:** −100% produces
  X scale −1 with Negative Scale on; with it off the tool reads zero and
  item scale stays one. Positive 50% yields 0.5. See IT-04–06.
- **M16-UI-Q04 — Answered for the requested case:** physical XYZ→ZXY at
  (30,45,60) preserves those Euler values and changes the evaluated world
  matrix/orientation (IT-PHY-01). Other order/angle/parent cases remain open.
- **PAR-Q05 — Partly:** combined Transform X translation measured (IT-02);
  rotation/scale portions and other axes remain open.
- **IT-Q01 — Answered:** separate fixture construction, GUI activation and
  live Tool Properties entry/viewport drag produce evaluated item channels.
  Script-only apply was not a substitute for this GUI route.
- **IT-Q02 — Open:** other rotation orders, Gimbal off, world displacement
  under parent transforms, and scale sensitivity at another zoom.
