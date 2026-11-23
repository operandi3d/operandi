# Animation phases 1-2: GUI checklist

For Jason or Codex (Claude's screen control can't click Operandi). QA app
named "Operandi Animation". Each step also once typed in the command line.

1. Layout > Animate: the tall timeline shows under the view; Model layout's
   clock button (bottom bar) shows and hides it.
2. Items mode, select a cube. Click Position X's status light at frame 0:
   it turns red. Drag the ruler to 24, type a new X: the light turns red
   again (Auto Key Animated). Scrub: the cube eases between.
3. Auto Key Off (timeline pop-up), move the cube at 30: the light shows
   yellow-in-red and "1 edited, not keyed" with Apply/Discard; Shift+Right
   snaps it back. Apply keeps it.
4. S with a label selected (click "Position X"), Shift+W/E keys, Shift+S.
5. `/` plays (Animate layout), right-click Play plays backwards; End
   Behaviour in the settings popover (gear).
6. Drag the timeline taller: marker row, then track rows. Click and drag
   keys, box-select, Shift+right-drag to scale, right-click > Linear,
   Copy / key time (middle-click ruler) / Paste Replace.
7. Middle-click the marker row, right-drag the marker for a range, turn
   Frame Limits on, step past its end: the time wraps.
8. Animate > Render Frame Range… and Playblast…: frames land in the
   folder shown; Cancel on the timeline stops.
9. Animate > Export USD Animation…: open the file in Blender, the ball
   bounces the same way.
10. Undo walks back each key step; scrubbing never appears in Undo.

## Graph editor (phase 3)

11. Animate > Graph Editor: the ball's Y curve shows sharp contacts and eased tops. Click a key: it's selected in the track row too.
12. Drag the top key up: the arc is higher; Undo takes it back in one step. Middle-drag moves values only, right-drag times only.
13. Select the contact key, Break, drag its out handle: the bounce changes, the key shows magenta.
14. Post menu > Repeat: the curve repeats past the last key; Cycle: the last key snaps to the first, a ghost square shows its own value.
15. Shadow Copy, edit, Swap, Swap: back where it started.
16. A, Shift+A, F, N over the graph; wheel zoom, Alt+middle-drag pan.
17. The Graph Editor opens as its own window (native title bar), a wide short strip over the lower part of the main window. Drag it onto another screen and resize it from any edge: it stays the size you give it (it used to grow taller by itself). Close and reopen: same place and size. Shortcuts (A, F, N over the graph, Cmd+Z) still work while it's in front.
18. Box Scale on, select keys: drag a box edge (time or value scales), Ctrl+corner (both); one undo.
19. Ctrl+Shift+drag selected keys: copies drop where you let go; originals stay.
20. Ctrl+Alt+right-drag a box: the view zooms to it.
21. Speed on: a second curve shows rate of change. Stacked: Ctrl+wheel changes rows, Shift+wheel scrolls them.
22. Retime on: click the strip to add marks, drag one: keys between marks stretch; right-click removes a mark.
23. In ▾ / Out ▾ set slopes (Out greyed while unified). Presets ▾ ▸ Save Envelope…, then apply it to another channel.
24. Animate menu ▸ Animation Editors: tall timeline plus the graph editor.

## Rig phase 4, steps 1-4

25. Select a cube, then a second one; Animate > Constrain > Position: the first jumps to the second (with Hierarchy > Compensation on it stays, then follows). One Cmd+Z removes it.
26. Its Position fields grey with "Driven by Position Constraint"; the status lights show the driven state.
27. Direction on a cone at a moving locator: it keeps aiming, no flips when the target passes straight overhead.
28. Hierarchy > Add Parent (child first, then parent), move the parent: the child follows; switch to World with the operandi.rig.dynamicParentSet line: it stays put.
29. Constrain A to B, then try B to A: "This would make B depend on itself", nothing changes.
30. Path: a curve, then Constrain > Path; key Percent 0 → 1 over 120 frames: constant speed, facing along the curve.
31. Select two cubes, Animate > Channel Links: the first is Driven, the second Driver. Pick scl.Z and pos.X, Add Link; drag the driver in X: the driven scales.
32. Relationship: pick the pair again with Relationship on, Add Link; type driven values and Key at two driver positions; the driven eases between them and holds outside.
33. Add User Channel (name, type angle) on the driven item, key it, link another item's rot.Z to it.
34. Animate > Schematic: constrained and linked items and their pieces as nodes; drag a node; drag from an item's output port to a Math modifier's A (Add ▾ > Math), then its out to another item's "+" and pick a channel; click a wire, Delete removes it; right-click a piece > Delete.
35. A position-constrained cube: its Move handle isn't offered ("Driven by Position Constraint"); Rotate's handle sits on the cube where it shows.
