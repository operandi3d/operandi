# Sculpt foundation: hands-on checklist

Branch `sculpt-foundation`. QA app: `Operandi sculpt-foundation <hash>.app`
in `/Volumes/Development/Operandi Builds`. Spec rules in brackets.

Setup: New Mesh, make a Sphere (or Cube) and Subdivide it a few times
(SDS Subdivide, D) so there are points to push; a plain grid works too.
Open the **Sculpt** toolbox tab (between Deform and Edit).

## Tab and panel
1. The tab shows Smooth, Push, Inflate, Move (two per row), a Tips row
   (Smooth Tip, Hard Tip) and the Lock Borders / Lock Corners checkboxes
   (Lock Corners on by default). Hover a brush: name, command, shortcut
   ("Unassigned"), "Ctrl: … · Shift: smooth". [SC-100]
2. Click Push: it lights in the accent colour; Tool Properties shows
   **Sculpt: Push** (one block: brush rows, a divider, Interpolation
   Step / Continuous Stroke / Curve Constraint, greyed Set Color / Color /
   Alpha, then Lock Borders and Lock Corners side by side) and **Smooth Brush**. Offset
   Distance is greyed while Offset Mode is Adaptive; switch to Absolute
   and Offset Amount greys instead. [SC-101, SC-102, SI §2]
3. Click Inflate: no Normal Mode row; Smooth Mode lists Shift Key, Low
   Pressure, Uniform; Density Mode lists Additive, Attenuated, Maximum
   Stroke and a greyed Maximum Map; Paint Mode, Set Color, Color and
   Alpha are greyed with a note on hover. Tick Lock Borders in the panel:
   the Sculpt tab's Lock Borders ticks too. Click Move: Sculpt Mode, Move
   Mode, Move Back Faces, Offset Amount (100 %), Smooth Amount, greyed
   Rigid UVs, then the locks. Smooth shows Sculpt Mode, Offset Amount
   (25 %), a greyed Soft Filter and the locks. [SI-04, SI-15, SI-16,
   SI-20..24, SPM-22..30]
4. With Push on, click Hard Tip: the tip section becomes **Hard Brush**
   with Soft Border instead of Shape Preset and Brush Settings. Back on
   the Smooth tip, click **Brush Settings**: a popover with In/Out Weight
   and In/Out Slope; changing one makes Shape Preset read Custom.
   [SC-31, SI-33, SI-34]

## Cursor and size
5. Over the mesh a ring follows the pointer, with a short line along the
   surface normal; off the mesh no ring. [SC-10, SC-11, SC-13]
6. Right-drag right/left: the ring grows/shrinks where the drag began and
   Size in the panel follows. A plain right-click (no drag) still opens
   the viewport menu. [SC-22, SC-104, UIC-60]
7. Middle-drag left/right: Offset Amount changes (shown next to the ring
   and in the panel). [SC-23]

## Strokes
8. Push: drag across the surface. Bumps rise as you drag, every frame,
   not just on release. Release, Cmd+Z once: the whole stroke goes; Edit
   menu said "Undo Push Stroke". Redo brings it back. [SC-40, SC-90, SC-92]
9. Ctrl+drag with Push digs in; Shift+drag smooths (Undo label "Smooth
   Stroke"). Alt+drag still orbits. [SC-60, SC-61, SC-64]
10. Inflate swells a curved area along each point's own normal; Ctrl
    deflates. [SC-122]
11. Smooth melts the bumps; Ctrl+Smooth smooths without shrinking as much.
    [SC-120, SC-63]
12. Move: press on the surface and drag; the patch under the ring follows
    the pointer in the screen plane (look from the side to lift it). Ctrl
    goes the opposite way. [SC-81, SC-123]
13. Start a press off the mesh: nothing happens. During a stroke, leave
    the mesh and come back: it pauses and carries on, one undo step.
    [SC-43]
14. Set Interpolation Step to 100 %: a quick stroke shows separate dots;
    tick Continuous Stroke: a solid line again. [SC-41, SC-42]

## Symmetry, locks, falloff
15. Turn Symmetry X on: a second ring (mirror colour) shows on the other
    side; a stroke changes both sides, one undo step. [SC-70, SC-71]
16. On an open grid: with Lock Corners on, smoothing over a corner leaves
    the corner point put; tick Lock Borders: the whole open edge stays.
    [SC-72, SC-73]
17. Turn on a Linear falloff (Deform tab) and Push across its ramp: the
    effect fades with the falloff. [SC-33]
18. Lock some points (geometry lock): brushes don't move them. [SC-03]

## Life and prefs
19. The brush stays on after a stroke; Q or Esc drops it (Space opens Mini
    Properties, UIC-66); picking Move (W) or any other tool drops it. [SC-93]
20. Change Push's Size, switch to Move and back: Push keeps its own
    settings. Quit and reopen: they're still there. [SC-95]
21. Save the scene after sculpting and reopen it: the shape is kept.
    [SC-94]

## Feel
22. On a heavily subdivided mesh (~100k points) with a 100 px brush the
    stroke stays smooth. Headless timing on the dev Mac (CPU side, no GPU
    upload): Push ~14 ms, Smooth ~10 ms, Move ~17 ms per frame; the first
    frame after release takes ~0.1 s (the stroke is recorded). [SC-110]
