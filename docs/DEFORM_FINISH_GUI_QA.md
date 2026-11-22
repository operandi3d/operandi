# Deform finish: GUI check

QA app: `~/Developer/Operandi-Builds/Operandi deform-finish <commit>.app` (built with OPERANDI_CYCLES=1). Start with a new scene, a Cube with Segments 8 x 8 x 8, Deform tab.

1. **Jitter** (Smooth flyout ▸ Jitter): Vertex mode, nothing selected, click in the view, then drag right. The whole cube roughens; Range X/Y/Z grows. Space drops; Cmd+Z takes it all back in one step.
2. **Jitter seed**: same Range, change Seed: a different pattern; back to 666: the first pattern again. Turn Enable Z off: nothing moves in Z, X/Y unchanged.
3. **Jitter + Symmetry X**: turn Symmetry X on, jitter: both halves mirror each other.
4. **Quantize**: Step X/Y/Z 0.25, click in the view: points snap onto a 0.25 m grid.
5. **Radial Align**: Edge mode, double-click an edge loop around the cube, click Radial, click in the view: the loop becomes a circle. Mode N-Sided, Sides 5: a pentagon. Drag left: Weight goes down and the loop eases back.
6. **Radial Align, polygons**: select a 3 x 3 patch of polygons on one face, Radial: the patch border becomes round and the inside stays flat and even.
7. **Linear Align**: select a run of edges along a bumpy row (jitter first), Linear: the run straightens between its ends. Uniform off: points keep their spacing.
8. **Radial/Linear greyed**: drop the selection: both buttons grey with a reason in the tooltip.
9. **Relax Mesh...**: jitter the cube, then Relax Mesh...: dialog Iterations 5, Constrain off; OK smooths it in one undo step. Constrain on: much smoother.
10. **Vortex**: Polygon mode, nothing selected, Vortex, click the middle of the top face, drag the rotate ring: the middle turns most, fading out at the cylinder's radius.
