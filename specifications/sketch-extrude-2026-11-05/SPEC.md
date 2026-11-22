# Sketch Extrude — behavior spec v0 (guide and settings only)

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05; the Modo 17.1 user guide for option names and intent (paraphrased). REA-assisted: no.

## Summary

Sketch Extrude extrudes the selected polygons along a path you draw in the
viewport: select polygons, pick the tool (Polygon tab), click to show the
handle, then drag from it; the extrusion follows the pointer.

## Settings after Reset (SE-D0)

| Setting | Default | Notes |
|---|---|---|
| Mode | **Sweep** | Sweep, Edit Path, Delete Knot, Delete Path, Uniform Spans, Straight |
| Move by Path | off | Edit Path only: dragging a control point also moves the points after it |
| Uniform | off | New geometry evenly spread between the polygon and the path end |
| Align to Path | **on** | New sections turn to follow the path; off keeps the original rotation |
| Precision | **30** | Screen pixels per new control span while sketching |
| Scale | 100 % | Thickness change from start to path end |
| Spin | 0° | Twist per span along the path |
| Make UVs | off, U Repeat off, V Repeat off | Guide: default fits new UVs in the existing UV space; U/V Repeat overlays each segment |
| Profile scale | 50 % | Profiles only |
| Profile Reverse X / Y | off / off | Profiles only |

One more setting (a UV rotation, 0) has no guide label. Switching the tool
on doesn't change the mesh. Align to Path read **off** before Reset and
**on** after it, so the last use's value carries over until Reset.

## Modes (guide)

- **Sweep:** drag a visible handle (cyan dot) to sketch the path.
- **Edit Path:** shows the path's control points; drag them to reshape.
- **Delete Knot:** click a control point to remove it.
- **Uniform Spans:** handles only at path ends; clicking one evens out the
  spacing of its path's knots.
- **Straight:** clicking an end handle straightens that path.
- **Delete Path:** a mode name read from Modo; the guide doesn't describe it.

## Rules

1. Only the settings and modes above are specified. Geometry (sections
   per span, placement along the path, Scale/Spin/Uniform) is open
   (Q-SE-01).

## Open questions and decisions

- **Q-SE-01 (decision pending)** Geometry for a drawn path: number of
  sections, their positions and rotation (Align to Path on/off), Scale,
  Spin, Uniform. Step 6 of the hands-on brief.
- **Q-SE-02** Make UVs: the guide's "default" option vs the read-back off
  value.
- **Q-SE-03** Delete Path mode (not in the guide).
