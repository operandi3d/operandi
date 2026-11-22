# Mesh Constraint button and popover: build spec v1, 2026-11-15

Behaviour only. Rule ids MC-xx. This spec covers the toolbar button, its popover, defaults, how it relates to Snapping, and SubD. What the constraint does to points is in the earlier spec SPEC-BACKGROUND-CONSTRAINT v1.1 (modes, Offset, acceptance cases BG-01..04). That spec still applies, except where this one differs.

## A. Move it out of Snapping
MC-01 Mesh constraint is not part of Snapping. Today Operandi puts background-constraint controls inside the Snapping popover. Take every one of them out. The Snapping popover holds only the rows in BEHAVIOR-SNAPPING-V2.md SN2-07. All mesh-constraint controls live in the Mesh Constraint popover (MC-05). Constraint Snapping (SN2-30b) stays in Snapping's geometry options; it is about snapping while dragging along one axis and has nothing to do with the mesh constraint.
MC-02 Separate on/off switches. Snapping and Mesh Constraint each have their own button, state and popover. Turning one on or off never changes the other (Operandi choice, MC-Q3).

## B. The button
MC-03 Mesh Constraint toolbar button. The button already sits in the toolbar but isn't wired up. Wire it:
- Click: toggles the mesh constraint on or off. On is orange (theme accent).
- Turning it on uses the last chosen mode. On a fresh install that is Background, so one click from new gives the background constraint.
- While on, the button's label shows the active mode's name ("Background"). While off, it reads "Mesh Constraint".
- Alt+click shows "Options" (existing modifier-buttons spec, MB) and opens the Mesh Constraint popover at the pointer.
- Also in the menu: Edit > Snapping > Mesh Constraints opens the same popover.
- No default keyboard shortcut.
MC-04 Esc. With no tool edit in progress, Esc turns the mesh constraint off. The button goes back to "Mesh Constraint", and its settings are kept for next time.

## C. The popover
MC-05 Row order, top to bottom.
1. Header "Mesh Constraint" and the pin circle (same popover rules as Snapping, SN2-06).
2. Constraint Mode: a strip of four buttons, (none), Guide, Background, Primitive, with the active one lit. (none) turns the constraint off. Operandi choice: Guide and Primitive greyed with a "Not built yet" tooltip. Only Background works in this build.
3. Group "Constrain to Background", shown only while Background is the mode: Offset (distance); Negative Offset (toggle: allows Offset below 0); Handle Constraint (toggle: also constrain the handles of curve-drawing tools); Double Sided (toggle: background polygons count from both sides); Geometry Constraint (dropdown in this order: Off, Screen Axis, Vector, Point).
Target Item (limit the constraint to one background item) appears only in the tool properties panel, after Geometry Constraint, not in the popover.

MC-06 Defaults: Mesh Constraint off; mode when first turned on Background; Offset 0 m; Negative Offset on (it only unlocks negative values); Handle Constraint on; Double Sided off; Geometry Constraint Screen Axis (Operandi choice); Target Item none (every background mesh counts). Settings persist across on/off and across launches.

## D. SubD
MC-07 Constrain to the smooth SubD surface (Jason 2026-11-15). When a background mesh is shown subdivided, the constraint uses the smooth subdivided surface the viewport is drawing at its current display level, not the cage. All three search modes: Screen Axis and Vector rays hit the smooth surface; Point finds the closest point on it; Offset follows the smooth surface normal at the hit. Non-subdivided background uses its polygons as they are. Reuse the evaluated surface the viewport and picking already use.
Acceptance: background = SubD cube (1 m cage, display level 2). Screen Axis, front view, Offset 0. A foreground point moved over the centre of the front face lands on the smooth surface, closer to the centre than the cage face's 0.5 m, matches the evaluated surface at that level within 1e-4 m, and sits visually on the drawn surface.

## E. How it sits with Snapping and tools
MC-08 Its own layer in the tool stack ("constraint"), separate from snapping. Operandi choice (MC-Q2): snapping runs first, constraint after, so a snapped point is then pushed onto the background surface. Both on = both work. Operandi doesn't force snapping off.
MC-09 Which tools: as in the earlier spec. Points a tool doesn't move are left alone.
MC-10 Tool presets can carry their own constraint later (e.g. Pen/Edge Slide with Point mode, a topology pen with Screen Axis + Double Sided). Leave room; don't build now.
MC-11 Default Constraint preference (None, Last Selected, Primitive, Background, Guide): later. Button always uses the last chosen mode.

## Open questions (use the Operandi choices above as provisional answers)
MC-Q1 Does the button label really show the mode name? MC-Q2 Snapping before or after the constraint? MC-Q3 Does turning the constraint on change Snapping? MC-Q4 Esc: clear in every case, or only when no tool is active? Earlier BG-Q1,Q3,Q4,Q6,Q7,Q8 stay open; Q2 settled by MC-06, Q5 by MC-07, Q9: Negative Offset starts on.
