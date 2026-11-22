# Drag creation of Cube, Sphere, Cylinder — behavior spec v1 (partial)

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-06. REA-assisted: no.

## Rules

1. **Base plane:** the first drag draws the base flat in the plane facing
   the camera (here XY, axis Z), giving a flat shape: Cube 4 points / 1
   polygon, Sphere and Cylinder a 24-sided disc (24/1). Size along the
   axis is 0.
2. **Cube is corner to corner:** the drag start and end are opposite
   corners; the centre is the drag's midpoint (D1: centre (0.409, −0.09,
   0), size 0.822 × 0.168).
3. **Sphere and Cylinder centre on the drag start:** the drag sets the
   radius (Sphere: radius X 0.516, Y 0.086; 24 sides, 12 segments, Globe
   method; Cylinder radii 0.822 × 0.168).
4. **Deterministic:** the same drag gives the same base (D2 = D1's base).
   Dropping the tool keeps the flat shape.
5. **Further drags made new shapes:** every later drag (on the base, on
   the square handle, on the green handle) started a new flat primitive
   and kept the old one; height was never added (D1: 8/2 after a second
   drag). How height is added interactively is open (Q-DC-01).

## Open questions and decisions

- **Q-DC-01 (hands-on)** The height stage: which handle or gesture adds
  height to the flat base. Automated drags never reached it.
- **Q-DC-02** Which plane the base uses in other views (Top, Front) and
  from other camera angles.
- **Q-DC-03** Sphere's unequal X/Y radii from one drag: confirm the
  rule that each axis follows the drag.
