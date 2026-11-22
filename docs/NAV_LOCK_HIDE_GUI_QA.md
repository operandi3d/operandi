# Navigation, lock and hide — GUI checklist

For Jason or Codex (branch `nav-lock-hide`). Start from a new scene with a
cube, pointer over the Perspective view.

## Navigation
1. Alt + middle drag in a circle: the view banks (rolls).
2. Alt + right drag: the view looks around from where it stands (the
   eye stays put, as on a swivel stool). Drag right: it turns to look
   right; drag up: it looks up. Letting go stops it at once. In Top view
   and in Camera view it works the same way (the camera turns in place).
3. Shift + Alt + right drag sideways and up/down: only the up/down part pans.
4. Ctrl + Alt + left drag (and Cmd + Alt + left): zooms; the point you
   pressed on stays put.
5. Ctrl + Alt + right drag a box around a corner: on release the corner
   fills the view.
6. `.` zooms in a step, `,` out. `/` starts a turntable; `/`, Esc or a click
   stops it. Tilt with Alt+middle, then `Shift+/`: the view levels.
7. Select one polygon, Cmd+A: the view faces it. Cmd+Shift+A: faces and
   frames it.
8. View > Navigate > Re-centre with the pointer on the cube (no key by
   default): orbiting now turns about that point. Bind it in the Input Map
   and try the key.

## Item lock
9. Right-click the cube in the Items list > Lock: a padlock appears after its
   name; Assembly > Lock shows Yes. Item mode Move: refused with a status
   message. Eye: refused. Polygon mode Move on a polygon: works.
10. Right-click > Unlock: padlock gone, the cube moves again.

## Geometry lock
11. Polygon mode, select two neighbouring polygons, J: they turn the locked
    colour and leave the selection. Select All and Loop skip them. With
    nothing selected, Move the mesh: the locked polygons stay.
12. Cmd+J: the two unlock, the rest lock. I: all unlocked.
13. View > Show Locked Mesh Items off: the colour goes; they still can't be
    selected.
14. Lock some polygons, save, reopen: still locked.

## Hide
15. Select two polygons, H: they vanish; the eye is unchanged; Select All
    selects the other four. Ctrl+H: only the two show. U: all back.
16. Hide some, save, reopen: everything shows. Hide then Cmd+Z: back.
17. On Mac, Cmd+H still hides the app (not the geometry).
