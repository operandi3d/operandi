# Unit Primitives and primitive size/position fields — behavior spec v1 (partial)

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05; the Modo 17.1 user guide for option names and intent (paraphrased). REA-assisted: no.

## Part 1 — Position, Size, Min and Max are one box

Every box-shaped primitive tool (Cube, Sphere, Ellipsoid … ) shows the
same box three ways per axis: Position (centre) c, Size s, and Min / Max.
For Cube, Size is the full width (max − min); for Sphere and Ellipsoid,
"Size" is the radius (half-width). Below, h = half-width (Cube: s/2;
Sphere/Ellipsoid: s).

1. **Editing Position** keeps the size and moves both bounds:
   min = c − h, max = c + h (UC02).
2. **Editing Size** keeps the centre: min and max move symmetrically
   (UC01, UC12). This holds in every Handle Mode (UC11, UC15); Handle Mode
   only affects dragging handles.
3. **Editing Min** keeps Max fixed and recomputes centre and size from the
   two bounds (UC03, UC13); **editing Max** keeps Min fixed (UC04, UC10,
   UC14 — also with Handle Mode "Resize").
4. **Order matters only through these rules:** Size 2 then Position 1
   gives the box 0…2, as does Position 1 then Size 2 (UC08, UC09). Min −1
   then Max 3 gives centre 1, Cube size 4 (UC10).
5. **No clamping.** Min above Max is accepted: Min 1 with Max 0.5 gives
   centre 0.75, Size −0.5 (UC05). A negative Size is accepted and swaps
   Min and Max (UC06). The geometry is the same as for the positive box
   (bounds sorted) with normals still outward (UC05, UC06).
6. **Size 0** collapses that axis; a Cube flattens to a single quad facing
   +X for Size X 0 (UC07).
7. **Axis** doesn't remap these fields: Size X is always world X (UC15).

## Part 2 — Unit Primitives

The primitive buttons have two quick variants (from the toolbar's
modifier bindings): **Ctrl+click** adds a unit primitive to the current
mesh; **Shift+click** makes a new mesh item holding it. The guide calls
these "unit primitives, about one metre in size". They don't open the tool
and don't use the tool's remembered settings.

8. **Into the current mesh** (Ctrl): the shape is added to the selected
   mesh's existing geometry (UP02: one quad + cube = 12 points, 7
   polygons). The item selection doesn't change (UP01).
9. **Tool settings ignored:** after leaving the Cube tool at Size X 3, two
   segments and a 0.1 radius, Unit Cube still makes the plain 1 m cube
   (UP03).
10. **New item** (Shift): a new mesh item named after the shape ("Cube",
    "Sphere", "Plane") is created and becomes the selected item; the
    previously selected mesh is unchanged (UP10–UP12).
11. **Shapes made** (all centred on the origin, axis Y, Face polygons):

| Variant | Points / polygons | Extent | Shape |
|---|---|---|---|
| Unit Cube | 8 / 6 | 1 × 1 × 1 | plain cube (UP01) |
| Unit Sphere | 266 / 288 | ±0.5 | 24 sides, 12 segments (UP04) |
| Unit Ellipsoid | 266 / 288 | ±0.5 | 24 sides, 12 segments (UP05) |
| Unit Cone | 25 / 25 | ±0.5 in X/Z, ±0.75 in Y | 24-sided base n-gon + 24 triangles to the apex (UP06) |
| Unit Cylinder | 48 / 26 | ±0.5 in X/Z, ±0.75 in Y | 24 quads + two 24-gon caps (UP07) |
| Unit Capsule | 242 / 264 | ±0.25 in X/Z, ±0.75 in Y | rounded ends (UP08) |
| Unit Toroid | 288 / 288 | ±0.75 in X/Z, ±0.25 in Y | all quads (UP09) |
| Plane Item | 4 / 1 | 1 × 1 at y = 0 | one quad facing +Y (UP12) |

    So "unit" is not a strict 1 m box: Cone, Cylinder and Capsule are
    1.5 m tall, the Toroid 1.5 m wide. Exact points per shape are in the
    cases.

## Open questions and decisions

- **Q-UP-01** Unit Cone/Cylinder/Capsule/Toroid segment counts and point
  order: only the counts and extents are summarised; the cases carry the
  full meshes. Cone/Cylinder/Capsule/Toroid tool specs should confirm the
  same layout as their tools at these settings.
- **Q-UP-02** What a Unit Primitive does with no mesh selected, or with
  several meshes selected.
- **Q-UP-03** Item variants for Cone, Cylinder, Capsule, Toroid (only
  Cube, Sphere and Plane probed). The Ellipsoid button's Shift variant is
  labelled "Teapot Item" in Modo 17.1's toolbar configuration (looks like a
  slip); Operandi should offer an Ellipsoid Item instead (decided below).
- **Q-UP-04** Do Unit Primitives make UVs, and in which map?
- **Q-UP-05** Undo: one step per Unit Primitive? (not scriptable)

## Decided (2026-11-05)

- **Q-UP-03 (Ellipsoid part) → decided:** the Ellipsoid button's Shift+click
  variant is "Ellipsoid Item" (a new mesh item holding a unit ellipsoid).
