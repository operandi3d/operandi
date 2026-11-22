# Tweak (Element Move) — behavior spec v1 (partial)

Provenance: observed in Modo 17.1v1 on 2026-11-03 by desktop control,
point positions read back to six decimals (oracles in `ORACLES.json`);
Modo help pages Element Move and Element Falloff for names and intent.
REA-assisted: no.

Tags: **observed** (oracle id given), **guide** (from the help, not
measured), **predicted** (not observed; question id given).

Fixtures used below:
- **G4**: 5 × 5 points, 1 m apart in the XY plane, from (-2,-2,0) to
  (2,2,0). Point index = 5 × row + column (rows/columns 0..4, row 0 at
  y = -2). 16 quads; quad index = 4 × row + column.
- **C2**: two unit cubes in one mesh, centers (0,0,0) and (3,0,0).

All observations were in a Front orthographic view.

## Activation

**TWK-01** T (no modifiers) activates Tweak. It stays active until another
tool is chosen or the tool is dropped. (key observed in the shortcuts
spec; staying active is guide)

**TWK-02** While active, hovering highlights the element a press would
grab. Nothing changes until the button goes down. (guide; thresholds
predicted, TWK-Q02)

## Picking

**TWK-03 Mode** has exactly seven entries, in this order: Auto, Auto
Center, Vertex, Edge, Edge Center, Polygon, Polygon Center. There is no
"Manual" entry and no separate Manual switch. (observed,
TWK-O-T4-02-mode-list)

- Auto / Auto Center: grab whichever element type is under the pointer.
- Vertex / Edge / Polygon (and their Center versions): grab only that type.
- The help says the Center versions measure the falloff from the
  element's center. The polygon measurement does not fit that reading:
  see TWK-12.

Default: the help says Auto. The tested session had Auto Center selected,
but that may have been left over from earlier use. Build Auto as the
default. (TWK-Q08)

**TWK-04** Auto priority: a vertex near the pointer wins over an edge, and
an edge wins over the polygon under the pointer. Use the same pixel
thresholds as the selection pre-highlight. (predicted, TWK-Q02)

**TWK-05** Only the frontmost surface can be grabbed in shaded views. In
wireframe, polygons can be grabbed only while **Always Raycast** is on.
(guide)

**TWK-06** With no selection, the element under the pointer is grabbed.
When something else is already selected, the behavior is not settled: in
the single trial (polygon 0 selected, drag on quad 10), no points moved,
and that trial ran just before a crash. Build it to ignore the selection
for now. (observed for the single trial, TWK-O-T4-11-selection;
general rule TWK-Q03)

## Click versus drag

**TWK-07 Press and drag** moves the grabbed element straight away.
(observed for vertex, edge and polygon, TWK-O-T4-04-*)

**TWK-08** The drag moves the element in the plane that faces the camera
and passes through the grab point, keeping the grab point under the
pointer. In the Front view all observed moves had zero Z, which agrees.
(Z = 0 observed; the pixel-to-distance rule and perspective are predicted,
TWK-Q04)

**TWK-09 Click without moving** shows a move handle at the element's center
(X and Y arrows plus the end-on Z control in Front view) and leaves the mesh
unchanged. For quad 10 of G4 the handle sat at (0.5, 0.5, 0). (observed,
TWK-O-T4-05-click) Dragging an arrow moves along that axis only, and
dragging the handle's center moves freely as in TWK-08. (predicted,
TWK-Q05)

**TWK-10** Each new press grabs whatever element is under the pointer and
starts a new tweak. (predicted, TWK-Q06)

## What moves (Range 0)

**TWK-11** At Range 0 only the grabbed element's points move, all by the
same offset: a vertex moves 1 point, an edge its 2 points, and a quad its
4 points. Every other point stays where it is. (observed: G4 point 18;
points 17 and 18; quad 10 = points 12, 13, 17, 18. TWK-O-T4-04-vertex-initial,
-edge, -polygon)

## Falloff (Range > 0)

**TWK-12** Each other point moves by weight × the grabbed element's offset.
Its weight comes from **d**, its distance to the **nearest point of the
grabbed element**, measured against **Range**. The grabbed element's own
points get weight 1. This fits all six falloff oracles:

- **Linear:** weight = 1 − d / Range, and 0 beyond Range. G4, Range 1.5,
  vertex at (0,0,0): neighbours along the axes (d = 1) get 0.333333 and
  diagonal neighbours (d = 1.414214) get 0.057191. (observed,
  TWK-O-T4-08-linear)
- **Smooth:** take the Linear weight w, then use 3w² − 2w³. The same pick
  gives 0.259259 and 0.009438. (observed, TWK-O-T4-08-smooth)
- **Polygon pick in Auto Center:** quad 10, Range 1.5. Its 4 corners get
  1, points 1 m from the nearest corner get 1/3, and points √2 m from it get
  0.057191. Point (0,-1,0) is 1.58 m from the quad's center but still
  moves, so distance is **not** measured from the center. (observed,
  TWK-O-T4-08-autocenter-polygon)
- **Ease-In, Ease-Out, Custom (In/Out):** use the same curves as the
  falloff spec's shape presets. (predicted)

**TWK-13 Depth.** The tests cannot tell whether d is a full 3D distance
or a distance across the screen. In the Front view, a cube's front and
back corners sit on top of each other, and in the C2 trial both of the
picked corner's pairs got weight 1. Two readings fit every number: (a) both
of those overlapping corners were grabbed and d is 3D, or (b) d is measured
across the screen. Build (a): 3D distance from the grabbed element's
points, grabbing the frontmost element only. (TWK-Q16)

**TWK-14** The falloff radius is drawn as a wireframe sphere, and a
right-button drag in the viewport changes Range. (guide; how much Range
changes per pixel is predicted, TWK-Q07)

## Connected Elements

**TWK-15** Tested on C2, picking the left cube's corner at (0.5, 0.5),
Range 5, Linear, Vertex mode: (observed)

| Setting | Result | Oracle |
|---|---|---|
| Ignore | Distance alone decides: points on the right cube move too, weights 0.37–0.6 (d = 2 to 3.16). | TWK-O-T4-09-ignore-vertex |
| Use Connectivity | The left cube is weighted the same as with Ignore; the right cube does not move at all. | TWK-O-T4-09-connectivity |
| Rigid Connections | All 8 left-cube points move by the full offset and the right cube stays put. The Range field is greyed out. | TWK-O-T4-09-rigid |
| Edge Loops | See TWK-16. | |

**TWK-16 Edge Loops**, G4, Range 0: (observed, TWK-O-T4-10-*)
- Grabbing an edge moves the whole edge loop it belongs to, all by the
  same offset. The horizontal edge 17–18 moved the whole y = 1 row
  (points 15..19).
- Grabbing a vertex moves only that vertex. Grabbing a polygon moves
  only its own points.
- On C2 with Range 5, Edge Loops weighted both cubes exactly like Ignore.
  That trial ran just after a dismissed warning and needs repeating, so for
  now treat Edge Loops plus Range as Ignore plus the loop.
  (TWK-O-T4-09-edge-loop)

Default setting: Ignore was the value seen in the session; the factory
default is unconfirmed. (TWK-Q08)

## Options

The "Seen" column lists values from the tested session. The factory
defaults are unconfirmed (TWK-Q08), so build these as the defaults.

| Option | Type | Seen | Notes |
|---|---|---|---|
| Position X / Y / Z | distance | 0 after a click | Shows the current tweak's offset. Typing a value should move the element by that offset (predicted, TWK-Q06; the test could not confirm this). |
| Mode | list of 7 | Auto Center | Build Auto as the default (guide). |
| Range | distance ≥ 0 | 0 m | 0 = only the grabbed element. |
| Connected Elements | Ignore / Use Connectivity / Rigid Connections / Edge Loops | Ignore | |
| Always Raycast | on/off | on | Lets polygons be grabbed in wireframe. |
| Shape Preset | Linear / Ease-In / Ease-Out / Smooth / Custom | Linear | |
| In / Out | number | — | Custom shape only. |
| Mix Mode | list | Multiply | How this combines with other falloffs; low priority. |
| Slip UVs | on/off | off | Low priority. |
| Morph | None / Transform / Keep Positions | None | Not needed until Operandi has morph maps. |

Pivot and Gimbal are shown but disabled. (guide)

## Still predicted (build, expect changes)

**TWK-17 Action center:** ignored; the move always comes from the grab.
(TWK-Q12)

**TWK-18 Symmetry:** with symmetry on, the mirror element also moves, by the
mirrored offset. An element on the mirror plane keeps its coordinate on the
mirror axis. This differs from plain Move, which only moves selected
elements, so it must be checked. (TWK-Q13)

**TWK-19 Snapping:** with snapping on, the dragged element lands on the snap
target. Which point of an edge or polygon snaps is open. (TWK-Q14)

**TWK-20 Empty space:** a press over empty space does nothing. (TWK-Q02)

**TWK-21 Undo:** each press-drag-release is one undo step and restores
every point it moved, including weighted ones. Esc during a drag cancels
that drag. The tool stays active. (TWK-Q15)

## Always

**TWK-22** Selection is unchanged after a tweak. (observed in every
oracle) Topology is unchanged: G4 stays 25 points and 16 quads, C2 stays 16
points and 12 quads, with the same polygon lists. (observed) Point and
polygon IDs stay stable (project rule).

## Acceptance cases

| Oracle | Checks |
|---|---|
| TWK-O-T4-04-vertex-initial / -edge / -polygon | Range-0 point sets (TWK-11) |
| TWK-O-T4-05-click | Handle at center, mesh unchanged (TWK-09) |
| TWK-O-T4-08-linear / -smooth | Weights (TWK-12) |
| TWK-O-T4-08-autocenter-polygon | Nearest-corner distance (TWK-12) |
| TWK-O-T4-09-ignore-vertex / -connectivity / -rigid / -edge-loop | TWK-15, TWK-16 |
| TWK-O-T4-10-edge / -vertex / -polygon | Edge Loops by pick (TWK-16) |
| TWK-O-T4-11-selection | Selection kept (TWK-06, provisional) |
| TWK-O-T4-01-retained, TWK-O-T4-02-mode-list | Option values seen, Mode list |

Compare **relative** offsets only: each point's offset ÷ the grabbed
element's offset, tolerance 1e-5. Do not use absolute distances or pixel
values (see the README caveat).
