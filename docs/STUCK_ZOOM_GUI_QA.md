# Stuck zoom — GUI QA

Branch `fix-stuck-zoom`. In Perspective views every zoom now uses the wheel's
Zoom to Surface rule (ZP-01..03): each step moves the eye and the orbit
centre toward the surface by the same fraction, stops 0.1 mm short and never
passes through. Before, only the wheel did; Ctrl+Alt drag zoom, the `.` / `,`
keys, the view header's Zoom button and the 3D mouse dollied toward the
fixed orbit centre and stopped 3 cm from it, which is where the zoom
"locked up" when that centre sat inside or behind the model.

| Input | Heads for |
|---|---|
| Wheel | the surface under the pointer (view centre with Wheel Zoom at Mouse Cursor off) |
| Ctrl+Alt drag | the surface under the press |
| `.` / `,`, header Zoom button, 3D mouse push/pull | the surface at the view centre |

Over empty space, in Orthographic/Top/Front views and with
`zoom_to_surface` off, all of them zoom about the orbit centre as before.
Symmetry has no effect on any of it.

Set-up: a Perspective view on a mesh (a Sub-D one if you have it). Frame it
(A), then pan so the model sits off to one side.

| ID | Steps | Expected |
|----|-------|----------|
| SZ-01 | Wheel in over a spot on the model, many notches. | The spot stays under the pointer; the view keeps closing in right up to the surface; never passes through. |
| SZ-02 | Frame (A). Ctrl+Alt drag zoom in, pressing over a spot on the model, for a long drag. | Same as SZ-01, about the press point. No stall a few cm out. Drag back: the view backs away from the same spot. |
| SZ-03 | Frame (A). Point the view centre at the model, press `.` repeatedly, then `,`. | Each `.` halves the way to the surface at the view centre; no stall; `,` backs out. |
| SZ-04 | Frame (A). Drag the view header's Zoom button. | Like SZ-03 (view centre). |
| SZ-05 | After any of SZ-01..04, orbit (Alt+drag) a little. | It turns about a point right by the surface you zoomed to, not about a far centre; no wild swing. |
| SZ-06 | Turn Symmetry X on (in Polygon mode, with and without a selection and a tool open). Repeat SZ-01..05. | Exactly as with Symmetry off. Try with the pointer on the symmetry plane too. |
| SZ-07 | If you have the 3D mouse: push in over the model in Perspective. | Closes in on the surface at the view centre instead of stopping. Top/Front views unchanged. |
| SZ-08 | Wheel / drag / `.` with the pointer over empty space, and in a Top view. | As before this branch. |
| SZ-09 | Deep zoom, then Frame (A). | Frames normally; no clipping or z-fighting afterwards. |

If SZ-01 (the wheel) still stalls in your scene, note which mesh, mode,
tool and whether Symmetry was on: the wheel already followed this rule
before the branch, and it reaches the surface in the tests with and without
Symmetry, so a stall there points at something scene-specific.
