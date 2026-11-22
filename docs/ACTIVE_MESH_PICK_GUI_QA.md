# Pick Other Meshes in Component Modes — GUI QA

Branch `active-mesh-pick`. Preference: Preferences > Selection > **Pick Other
Meshes in Component Modes** (off by default).

Setup: a scene with two meshes, one partly in front of the other (e.g. two
cubes, one moved toward the camera so it covers part of the other). Make the
back one active (click it in Item mode or in the Items list).

## Off (default)

1. Polygon mode: click a polygon of the front (inactive) mesh where the active
   mesh is behind it. The active mesh's polygon behind it is selected; the
   active mesh does not change.
2. Polygon mode: click the inactive mesh where nothing of the active mesh is
   behind. Nothing changes (selection kept); the status line names the
   preference.
3. Click empty space: the component selection clears as before.
4. Vertex and Edge modes: points and edges of the active mesh behind the
   front mesh can be clicked and pre-highlight; nothing on the front mesh
   pre-highlights or picks.
5. Paint-drag select across both meshes: only the active mesh's elements
   are added.
6. Tweak (Vertex/Edge/Polygon): grabs only the active mesh, also through
   the front mesh.
7. Edge mode double-click: loops only on the active mesh.
8. Closest rollovers + Mouse Regions Trigger Selections on: right-click and
   rollovers only find the active mesh's elements.
9. Polygon Selection Fill (Shift+Cmd click) works on the active mesh through
   the front mesh.
10. Hide the active mesh (or select nothing in Item mode): component clicks
    pick any mesh again, so you're never stuck.

## On

11. Tick the preference: clicking the front mesh in a component mode makes it
    the active mesh (the old behaviour), and the front mesh hides what's
    behind it again.

## Unchanged

12. Item mode: clicking either mesh selects it, with the preference on or off.
13. Quit and relaunch: the checkbox keeps its state.
