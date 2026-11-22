# Preferences: Units, Grid and Work Plane (behaviour spec v1, 2026-11-04)

Behaviour-only. "Not yet measured" = factory value still to be read (v2);
use a provisional value and log it. Settings apply at once and persist per
user.

## Distance Units (Input > Distance Units)

| Setting | Type | Default | Behaviour |
|---|---|---|---|
| Unit System | dropdown: SI, Metric, English, Game Units, Unitless | not yet measured | Unit family used to show and read every distance field. SI and Metric are metre-based (mm, cm, m, km); English uses mils, inches, feet, yards, miles; Game Units are an arbitrary unit sized by Meters per Game Unit; Unitless shows plain numbers where 1 = 1 m. |
| Default Unit | dropdown; choices depend on Unit System | not yet measured | Unit assumed when a typed number has no unit. |
| Inch Scale Cutoff | whole number (inches) | 36 | English only: lengths up to the cutoff are shown in inches (27 in → 27"); longer ones as feet and inches (42 in → 3' 6"). |
| Meters per Game Unit | number | not yet measured | Size of one game unit in metres (e.g. 1.375). |
| Input As Current Unit | checkbox | not yet measured | When on, a number typed without a unit is taken in the unit currently shown in that field. |

Other unit settings on this panel (rated later/never for Operandi):
force, acceleration, speed and mass units (dynamics), light units
(Radiometric/Photometric).

## Light and colour, percentages (same panel)

| Setting | Type | Default | Behaviour |
|---|---|---|---|
| Color Unit System | dropdown: Floating Point, Percentage, Integer, Hexadecimal | not yet measured | How colour values are shown and typed: 0–1 floats (can exceed 1), 0–100 %, 0–255, or hex (00–ff per channel). |
| Factor or Range | dropdown: Percentage, Float | not yet measured | Whether 0–1 factors are shown as 0–100 % or 0.0–1.0. |

## Accuracy (same panel)

| Setting | Type | Default | Behaviour |
|---|---|---|---|
| Coordinate Rounding | dropdown: None, Normal, Fine, Fixed, Forced Fixed | not yet measured | How a pointer position on the work plane becomes a 3D coordinate. None: raw values. Normal: rounded to clean values whose step grows and shrinks with zoom (a move of 2–3 px per step). Fine: like Normal but about one step per pixel. Fixed: like Normal, but neither the step nor the grid gets finer than Fixed Increment (zooming out still coarsens). Forced Fixed: step and grid always equal Fixed Increment at every zoom. |
| Fixed Increment | distance | not yet measured | Step used by the two Fixed modes (e.g. 10 mm). |

## Coordinate System (same panel)

| Setting | Type | Default | Behaviour |
|---|---|---|---|
| Up Axis | dropdown: X, Y, Z | not yet measured (Y expected) | Which axis counts as up for the app. Also used when importing files from apps with a different up axis. |

## Grid and Work Plane (Input > Grid and Work Plane)

| Setting | Type | Default | Behaviour |
|---|---|---|---|
| Grid Type | dropdown: Fixed, Dynamic | not yet measured | Fixed: grid squares have one size. Dynamic: spacing follows zoom, and the view shows the current square size in its lower-right corner. |
| Size | distance (enabled only for Fixed) | not yet measured | Square size of the fixed grid. When zoomed out so far that lines would crowd, the grid switches to dynamic spacing. |
| Preferred Plane | dropdown of the three axis planes | not yet measured | Plane the work plane leans toward when Bias is above 0. |
| Preferred Plane Bias | percent | not yet measured (0 expected) | Normally the work plane follows the view (the plane most facing the viewer). Raising the bias makes it favour the preferred plane more; very high values keep it there almost always. |
| Lock Preferred Plane | checkbox | not yet measured | Keeps the work plane on the preferred plane however the view is rotated. |
| Locked Plane Offset | distance | not yet measured | Offset of the locked plane from the origin along its normal. |
| Plane and Grid Increments | dropdown (choices not yet listed) | not yet measured | How grid density steps (which subdivisions are added or removed) as you zoom. |

## Related display settings (Display > OpenGL)

| Setting | Type | Default | Behaviour |
|---|---|---|---|
| Grid Visibility | checkbox | on | Whether new 3D views show the ground grid. |
| Flatness of Perspective | number | not yet measured | Strength of perspective in 3D views; higher is flatter, lower is more distorted (like field of view). |
| Viewport Fit Factor | number | 1.5 | Margin used by Fit All / Fit Selected: 1 makes the bounding box touch the view's shorter side; 1.5 leaves room around it. |

## Open questions

- UNI-Q1: factory values above; Default Unit choices per Unit System.
- UNI-Q2: exact rounding step rule for Normal and Fine.
- GRD-Q1: choices of Preferred Plane and Plane and Grid Increments.
- GRD-Q2: whether Fixed grid size also drives coordinate rounding.
