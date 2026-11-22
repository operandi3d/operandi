# Set Position — behavior spec v1

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05; the Modo 17.1 user guide for option names and intent (paraphrased). REA-assisted: no.

## Summary

Set Position gives every affected point the same coordinate on one axis.
Options: **Axis** (X, Y, Z), **Value** (distance), **World Space**
(on/off), **Work Plane** (on/off). Other coordinates never change, and
nothing else changes (no new points or polygons).

## Rules

1. **The coordinate:** each affected point's coordinate on Axis becomes
   Value (ST01–ST03). Points already at Value don't move (ST02, point 1).
2. **Affected points:**
   - Vertex mode: the selected points; nothing selected → every point of
     the mesh (ST04).
   - Polygon mode: the points of the selected polygons (ST05); nothing
     selected → every point (ST07).
   - Edge mode: the end points of the selected edges (ST06).
   - Item mode: every point of the selected mesh (ST16).
3. **Default space:** with both switches off, Value is in the mesh item's
   own (local) coordinates; the item's position is ignored (ST08: item at
   (1,2,3), X set to 0 gives local x = 0).
4. **World Space on:** Value is a world coordinate on the world axis. The
   point is placed so its world coordinate on that axis equals Value and
   its other two world coordinates stay the same, then stored back in
   local coordinates. Item moved by +1 in X: world X 0 means local X −1
   (ST09). Item rotated 90° about Z: world X 0 sets the point's local Y to
   0 (ST10). Item scaled 2: world X 0.5 means local X 0.25 (ST11).
5. **Work Plane on:** Value is measured along the work plane's own axis
   from the work plane's centre, keeping the point's other two work-plane
   coordinates. The default work plane equals world space (ST12). Work
   plane centred at z = 1: Z 0.5 puts the point at z = 1.5 (ST13). Work
   plane rotated 90° about Z: its X axis is world +Y, so X 0.5 sets world
   Y to 0.5 (ST15). With the switch off the work plane is ignored (ST14).
6. **Selection after:** unchanged, mode unchanged.
7. **Not observed:** both switches on together; a work plane on a moved
   item; the dialog's remembered values (it opens a dialog in the UI).

## Open questions and decisions

- **Q-ST-01** World Space and Work Plane both on: which wins.
- **Q-ST-02** The dialog's first-use defaults and whether it remembers the
  last values (dialog only; not scriptable without opening it).
- **Q-ST-03** Several meshes selected in Item mode.
