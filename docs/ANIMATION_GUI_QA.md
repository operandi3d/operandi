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

## Rig phase 5 (joints, binding, weights)

36. Timeline Setup button (setup-pose icon) on: every 3D view gets a yellow outline; a keyed cube shows its setup pose and dragging it doesn't add keys. Off: the animated pose again.
37. Skeleton button (skeleton-tool icon): its options window opens. Over a cylinder in a side view, click three times along it: a chain of joints halfway through the cylinder, each with rings and a bone to the next. Shift+click starts a new chain; click an existing joint, then click again: a branch from it. Click-drag places the joint as you drag; one Cmd+Z per click.
38. Symmetry X: a joint off the middle draws its mirror at once, named …_L / …_R. Edit mode drags a joint (Compensation on: the rest of the chain stays); Insert splits a bone; Delete removes a joint, children stay where they are.
39. Select the cylinder, then the root joint; Setup on; Bind: the Bind dialog (Heat default). OK: the cylinder gets a Skin row in its Mesh Stack (skin-mesh icon). Setup off, rotate a joint: the cylinder bends, nothing left behind. Shift+click Bind unbinds.
40. Bind a mesh with a loose point (Heat): the dialog stays open with the reason and "Use Smooth (Distance) instead"; the loose point is selected.
41. Select a bound joint: its weight map becomes the selected map; Vertex Map shading shows it; Vertex Map Tools Paint changes it and the other joints' maps lower (totals 100%).
42. Timeline Weights ▾: Smooth Selected, Normalize, Mirror (left to right), Split; each one Cmd+Z.
43. File > Export USD the bent cylinder; open it in Blender: an armature and the same bend; play: same animation.

## Rig phase 6 (IK and posing)

44. Draw a three-joint leg with a slight knee bend; Setup on; select hip, knee, ankle in order; timeline IK ▾ > Apply IK (Two-Bone) (two-bone icon): a goal box (green on +X, red on −X, blue in the middle) at the ankle. Setup off, drag the goal: the leg follows, the knee keeps bending the same way; pull it out of reach: the leg straightens, no stretch.
45. IK ▾ > Create IK Up Vector (up-vector icon): a pole appears in front of the knee; move it: the knee points at it. Rig channels: Orient spins the knee, Soft eases the last bit of reach, Stretch reaches the goal.
46. A five-joint tail: select root then end, IK ▾ > Apply IK (Chain) (chain icon); drag its goal around: every bone keeps its length.
47. Joint limits: on the knee run `operandi.ik.jointSet enableLimits:true useLimitsX:true minX:0 maxX:90`; select the knee: a thin fan shows the allowed arc; the solved knee stops at it.
48. Animate the goal 0–24, at 24 IK ▾ > Switch to FK; key a hip turn at 48; scrub 16–32: no pop at 24. At 48 IK ▾ > Switch to IK: no pop. IK ▾ > IK/FK: Set Keys / Delete Keys work at the current frame.
49. Full body: a biped's hips, Setup on, IK ▾ > Assign IK (Full Body); select a hand, Add Goal (goal icon); select each foot, Add Position Task; Setup off; drag the hand goal: the arm reaches, the feet stay; Translate Root on (rig channel): the hips lean towards the hand.
50. Select the hips, timeline Pose: white circles on every joint, the one under the pointer yellow. Right-click both feet (red), drag the head down: the knees bend, the feet stay. Shift+drag twists, middle-drag turns one joint, drag empty space moves the whole body; each drag is one Cmd+Z. With Auto Key on, the moved joints get keys.
51. Bake Selected on any joint of an IK leg: the leg plays the same with the solver switched to FK. File > Export USD the skinned IK leg and open it in Blender: same motion.

## Rig rows in the Items list

52. Constrain a cube to another: the Items list shows "Position Constraint" under the cube (constraint icon). Click it: the cube is selected and the row marked; double-click renames it (Cmd+Z undoes); its eye turns it off and on; right-click shows Rename, Disable, Delete, Select Item.
53. Bind a mesh to a skeleton: a "Skin" row under the mesh; open it: one row per joint. A joint row's eye turns that joint's influence off; clicking it selects the joint. Right-click the Skin: Enable/Disable, Bake Weights, Duplicate and Freeze, Delete (Unbind, greyed outside Setup).
54. Two-bone IK on a leg: "Two-Bone IK" shows under the hip joint (two-bone icon); its goal and the IK root are item rows.

## Rig tab

55. Switch to the Animate layout: the toolbox strip has a Rig tab (after Curve). It lists Setup, Skeleton, Pose; Constraints (a flyout of the nine kinds), Parent, Unparent, Add Parent, Compensation; Channel Links, Schematic, Bake Selected; Bind (Shift+click Unbind), Add Joints to Bind and the weight tools; the IK buttons (Two-Bone, Chain, Full Body, Add Goal, tasks, Up Vector with Shift+click Remove, Switch IK/FK, Set Keys with Shift+click Delete Keys, selects, Match, Update Rest Pose, Disable, Remove). Each button runs its command; greyed ones say why.
56. The timeline no longer has the Skeleton, Pose, Bind, Weights and IK buttons (they are on the Rig tab); Setup and the phase-4 rig buttons stay there.
57. In the Model layout the Rig tab isn't in the strip; `operandi.toolbox.tab rig` opens it there too.
