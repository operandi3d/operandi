# X-Ray / Ghost buttons — GUI check

QA build from branch `xray-buttons`. Spec:
`specifications/xray-toggle-2026-11-13/BEHAVIOR-XRAY-TOGGLE.md`.

1. New scene: a cube in front of a sphere, both in one mesh layer or two
   items. Each 3D view's header shows three round buttons left of Pan:
   Normal (solid ball, lit orange), Ghost (faint ball), X-Ray (outline with
   a small cube). Hover each: the tooltip names it, says what it does and
   mentions Alt+click.
2. Advanced style, cube selected (active). Click X-Ray: the cube's front
   faces vanish, its back faces stay, the sphere shows through. X-Ray is
   lit. (XR-01)
3. Hold Alt: the lit button now shows the inactive meshes' mode (Normal).
   Alt+click Ghost with the sphere inactive: the sphere goes semi-opaque,
   the cube stays X-Ray. Release Alt: X-Ray lit again. (XR-02)
4. Display ▸ Wireframe: the three buttons grey out; clicking says "Not with
   the Wireframe style". Viewport Properties ▸ Active Meshes ▸ Transparency
   Mode is greyed too. (XR-03)
5. Split the view: change X-Ray in one view only; the other keeps its own.
   Quit and relaunch: each view kept its mode.
6. Type `Toggle X-Ray` in the command line twice: X-Ray, then Normal.
   `operandi.view3d.toggleXRay` does the same. Bind it in the Input Map and
   press the key over a view that isn't active: that view toggles. (XR-04)
7. Viewport Properties ▸ Transparency Mode follows the buttons, and the
   buttons follow the dropdown.
8. Narrow a view to under ~200 px wide: the three buttons drop out before
   Pan/Zoom/Maximise/gear do.
