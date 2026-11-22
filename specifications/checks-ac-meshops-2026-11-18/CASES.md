# Action Center and MeshOps checks (2026-11-18): cases in words

Provenance: research-side behaviour checks. Inputs are our own meshes (inputs.json). No reference results are included; expected behaviour is stated in words. REA-assisted: no.

## Action Center (AC-xx)

Mesh: two islands, a tilted two-quad strip and a flat quad, in a mesh item moved to (2, 0, 1) and turned 30 degrees about Y; all three polygons selected (inputs.json). Tools and amounts: Scale (factors 2, 1, 1), Rotate (30 degrees about Z), Move (0.25 m along world X), Smooth Shift (offset 0, Scale 50%), Thicken (offset 0.1 m, Scale 50%). Each tool runs once per Action Center.

| Case | Expected behaviour |
|---|---|
| AC-scale-None | held: depends on a viewport click |
| AC-rotate-None | held: depends on a viewport click |
| AC-move-None | a straight move of 0.25 m along world X (every Action Center gives the same move here) |
| AC-smoothshift-None | held: Smooth Shift/Thicken Scale centre waits on the hands-on check |
| AC-thicken-None | held: Smooth Shift/Thicken Scale centre waits on the hands-on check |
| AC-scale-Automatic | held: depends on a viewport click |
| AC-rotate-Automatic | held: depends on a viewport click |
| AC-move-Automatic | a straight move of 0.25 m along world X (every Action Center gives the same move here) |
| AC-smoothshift-Automatic | held: Smooth Shift/Thicken Scale centre waits on the hands-on check |
| AC-thicken-Automatic | held: Smooth Shift/Thicken Scale centre waits on the hands-on check |
| AC-scale-Selection | about the middle of the selected points' bounding box, world axes |
| AC-rotate-Selection | about the middle of the selected points' bounding box, world axes |
| AC-move-Selection | a straight move of 0.25 m along world X (every Action Center gives the same move here) |
| AC-smoothshift-Selection | held: Smooth Shift/Thicken Scale centre waits on the hands-on check |
| AC-thicken-Selection | held: Smooth Shift/Thicken Scale centre waits on the hands-on check |
| AC-scale-SelectionCenterAutoAxis | held: depends on a viewport click |
| AC-rotate-SelectionCenterAutoAxis | held: depends on a viewport click |
| AC-move-SelectionCenterAutoAxis | a straight move of 0.25 m along world X (every Action Center gives the same move here) |
| AC-smoothshift-SelectionCenterAutoAxis | held: Smooth Shift/Thicken Scale centre waits on the hands-on check |
| AC-thicken-SelectionCenterAutoAxis | held: Smooth Shift/Thicken Scale centre waits on the hands-on check |
| AC-scale-Origin | about the world origin, world axes |
| AC-rotate-Origin | about the world origin, world axes |
| AC-move-Origin | a straight move of 0.25 m along world X (every Action Center gives the same move here) |
| AC-smoothshift-Origin | held: Smooth Shift/Thicken Scale centre waits on the hands-on check |
| AC-thicken-Origin | held: Smooth Shift/Thicken Scale centre waits on the hands-on check |
| AC-scale-Local | each connected island about its own centre, along its own axes |
| AC-rotate-Local | each connected island about its own centre, along its own axes |
| AC-move-Local | a straight move of 0.25 m along world X; on this mesh Local gives the same result |
| AC-smoothshift-Local | held: Smooth Shift/Thicken Scale centre waits on the hands-on check |
| AC-thicken-Local | held: Smooth Shift/Thicken Scale centre waits on the hands-on check |
| AC-scale-Border | held: waiting on the hands-on check |
| AC-rotate-Border | held: waiting on the hands-on check |
| AC-move-Border | a straight move of 0.25 m along world X (every Action Center gives the same move here) |
| AC-smoothshift-Border | held: Smooth Shift/Thicken Scale centre waits on the hands-on check |
| AC-thicken-Border | held: Smooth Shift/Thicken Scale centre waits on the hands-on check |
| AC-scale-Pivot | about the item's pivot (here the item position) |
| AC-rotate-Pivot | about the item's pivot (here the item position) |
| AC-move-Pivot | a straight move of 0.25 m along world X (every Action Center gives the same move here) |
| AC-smoothshift-Pivot | held: Smooth Shift/Thicken Scale centre waits on the hands-on check |
| AC-thicken-Pivot | held: Smooth Shift/Thicken Scale centre waits on the hands-on check |

Found so far on main f4aa4fe8: Selection uses the average of the points instead of the bounding-box middle (shows with Rotate); Local differs for Scale and Rotate; Pivot is not built.

## MeshOps (MOP-xx)

| Case | Setup | Expected behaviour |
|---|---|---|
| MOP-MM-M01_default | Merge Meshes op on T; sources S1; options reset values | T shows a copy of each source mesh; sources placed by their world transforms, relative to T; materials kept from the sources |
| MOP-MM-M02_world0 | Merge Meshes op on T; sources S1; options {'World Space': False} | T shows a copy of each source mesh; sources placed in T's own space, ignoring their item positions; materials kept from the sources |
| MOP-MM-M03_useWorld0 | Merge Meshes op on T; sources S1; options {'Use World Transforms': False} | T shows a copy of each source mesh; sources placed by their world transforms, relative to T; Use World Transforms off; materials kept from the sources |
| MOP-MM-M04_both0 | Merge Meshes op on T; sources S1; options {'World Space': False, 'Use World Transforms': False} | T shows a copy of each source mesh; sources placed in T's own space, ignoring their item positions; Use World Transforms off; materials kept from the sources |
| MOP-MM-M06_three_all | Merge Meshes op on T; sources S1, S2, S3; options reset values | T shows a copy of each source mesh; sources placed by their world transforms, relative to T; materials kept from the sources |
| MOP-MM-M08_targetBase | Merge Meshes op on T; sources S1; options reset values | T shows a copy of each source mesh; sources placed by their world transforms, relative to T; materials kept from the sources |
| MOP-MM-M09_hier_on | Merge Meshes op on T; sources S1; options {'Hierarchy': True} | T shows a copy of each source mesh; sources placed by their world transforms, relative to T; children of the sources are merged too; materials kept from the sources |
| MOP-MM-M10_hier_off | Merge Meshes op on T; sources S1; options reset values | T shows a copy of each source mesh; sources placed by their world transforms, relative to T; materials kept from the sources |
| MOP-MM-M11_rotY90 | Merge Meshes op on T; sources S1; options reset values | T shows a copy of each source mesh; sources placed by their world transforms, relative to T; materials kept from the sources |
| MOP-BO-Boolean-Union | Boolean op on A, mode Union, driver B overlapping A's +X+Y+Z corner | A shows the Union of A with B as a live op; same result as the one-shot Boolean with the same mode |
| MOP-BO-Boolean-Intersect | Boolean op on A, mode Intersect, driver B overlapping A's +X+Y+Z corner | A shows the Intersect of A with B as a live op; same result as the one-shot Boolean with the same mode |
| MOP-BO-Boolean-Subtract | Boolean op on A, mode Subtract, driver B overlapping A's +X+Y+Z corner | A shows the Subtract of A with B as a live op; same result as the one-shot Boolean with the same mode |
| MOP-BO-Boolean-Add | Boolean op on A, mode Add, driver B overlapping A's +X+Y+Z corner | A shows the Add of A with B as a live op; same result as the one-shot Boolean with the same mode |
| MOP-BO-Boolean-UnionAll | Boolean op on A, mode Union All, driver B overlapping A's +X+Y+Z corner | A shows the Union All of A with B as a live op; same result as the one-shot Boolean with the same mode |
| MOP-BO-SolidDrill-Tunnel | Solid Drill op on A, mode Tunnel, driver B overlapping A's +X+Y+Z corner | A shows the Tunnel of A with B as a live op; same result as the one-shot Solid Drill with the same mode |
| MOP-FZ-F1_circle_default | Freeze op on top of a circle curve (radius 0.5, XZ plane); options reset values | Freeze op on top of the stack bakes everything below it into plain polygons (options as listed); Operandi has no Freeze op yet |
| MOP-FZ-F4_circle_bezierOff | Freeze op on top of a circle curve (radius 0.5, XZ plane); options {'Bezier': False} | Freeze op on top of the stack bakes everything below it into plain polygons (options as listed); Operandi has no Freeze op yet |
| MOP-FZ-F5_circle_fixed | Freeze op on top of a circle curve (radius 0.5, XZ plane); options {'Fixed Spans': True} | Freeze op on top of the stack bakes everything below it into plain polygons (options as listed); Operandi has no Freeze op yet |
| MOP-FZ-G5_subd_freeze | Freeze op on top of a unit cube op, then Set Polygon Type: Subdivision; options reset values | Freeze op on top of the stack bakes everything below it into plain polygons (options as listed); Operandi has no Freeze op yet |
| MOP-FZ-G7_psub_freeze_off | Freeze op on top of a unit cube op, then Set Polygon Type: Catmull-Clark; options {'Catmull-Clark': False} | Freeze op on top of the stack bakes everything below it into plain polygons (options as listed); Operandi has no Freeze op yet |
