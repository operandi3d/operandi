# Zoom to Surface — GUI QA

> Since `fix-stuck-zoom` (2026-11-14), drag zoom, the `.` / `,` keys, the
> header Zoom button and the 3D mouse follow the wheel's rule too, so ZQ-06
> no longer holds. See `STUCK_ZOOM_GUI_QA.md`.

Branch `zoom-to-surface`, spec ZOOM-PIVOT (ZP-01..04). In Perspective views
the mouse wheel zooms toward the surface under the pointer: the eye and the
orbit centre both move toward it by the same fraction each notch, so the
spot stays under the pointer and the view no longer stops at the old 3 cm
limit short of the model. The near clip shrinks with the zoom, so the
surface stays drawn until the eye is 0.1 mm away. Behind
`NavigationOptions::zoom_to_surface` (default on, no UI yet). Ctrl+Alt drag
zoom, the `.` / `,` keys, ortho views, SpaceMouse, box zoom and Frame All
are unchanged.

Set-up: a Perspective view on a box or a larger scene. Orbit so the orbit
centre is not on the model (e.g. pan the model to one side of the view).

| ID | Steps | Expected |
|----|-------|----------|
| ZQ-01 | Wheel-zoom in over a point on the model, many notches. | The point under the pointer stays under it; the view keeps closing in well past 3 cm and never passes through the surface. |
| ZQ-02 | Keep zooming in at ZQ-01's spot until it stops. | The surface stays drawn (no clipping hole) down to a fraction of a millimetre, with no flicker; the wheel then does nothing more. |
| ZQ-03 | After ZQ-01, orbit (Alt+drag) a little, then wheel out. | The orbit centre has moved toward the surface (orbit turns about a nearer point than before); wheeling out backs straight away from the spot. |
| ZQ-04 | Preferences > Input: turn Wheel Zoom at Mouse Cursor off; wheel-zoom with the pointer off to the side. | Zoom heads for the surface at the view centre, not the pointer. Turn it back on afterwards. |
| ZQ-05 | Wheel-zoom with the pointer over empty space. | Same as before this branch: zoom about the orbit centre, the pointer's spot on the ground plane through it held in place. |
| ZQ-06 | Ctrl+Alt drag zoom and `.` / `,` over the model. | Same as before this branch (they still stop at 3 cm from the orbit centre). After a deep wheel zoom they don't jump the view back out. |
| ZQ-07 | Wheel-zoom in an Orthographic, Top or Front view. | Unchanged from before this branch. |
| ZQ-08 | Deep wheel zoom (ZQ-02), then Frame All (A). | The scene frames normally; distant faces show no z-fighting afterwards. |
| ZQ-09 | Fast wheel zoom on a 100k+ polygon mesh. | No hitches beyond plain orbiting (one pick per wheel event). |
