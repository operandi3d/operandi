# Default keyboard shortcuts — behavior v1 (partial)

Observed in Modo 17.1v1 on 2026-11-02 by desktop control (Codex), results
read back from Modo and UI. REA-assisted: no.

**KEY-01 The following unmodified keys invoke the listed action in the tested context.**
The input is an identity unit cube centered at the origin: eight points at
all combinations of ±0.5 m and six quads. Polygon tests select the +Z face,
Edge tests its bottom edge, and Vertex tests its bottom-left point. No
pointer drag accompanies a key press. Front orthographic, 624 px/m
(±1 px measurement); the 1 m face spans 624 px. Key activation was tested at
one zoom; no pixel-sensitive keyboard behavior is inferred. Each oracle
contains exact input, preview mesh and selection.

| UI tool / command | Key | macOS modifiers | Tested context | Verified by press | Oracle |
|---|---|---|---|---|---|
| Move | w | none | polygon | yes | KEY-O-KEY-P01 |
| Rotate | e | none | polygon | yes | KEY-O-KEY-P02 |
| Scale | r | none | polygon | yes | KEY-O-KEY-P03 |
| Transform | y | none | polygon | yes | KEY-O-KEY-P05 |
| Element Move | t | none | polygon | yes | KEY-O-KEY-P06 |
| Polygon Bevel | b | none | polygon | yes | KEY-O-KEY-P07 |
| Slide unavailable for one selected polygon | s | none | polygon | yes | KEY-O-KEY-P13 |
| Edge Bevel | b | none | edge | yes | KEY-O-KEY-P14 |
| Slide (Edge) | s | none | edge | yes | KEY-O-KEY-P15 |
| Vertex Bevel | b | none | vertex | yes | KEY-O-KEY-P16 |
| Slide (Vertex) | s | none | vertex | yes | KEY-O-KEY-P17 |
| Vertex selection mode | 1 | none | polygon | yes | KEY-O-KEY-P18 |
| Edge selection mode | 2 | none | polygon | yes | KEY-O-KEY-P19 |
| Polygon selection mode | 3 | none | vertex | yes | KEY-O-KEY-P20 |
| Items selection mode | 5 | none | polygon | yes | KEY-O-KEY-P21 |
| More selection refusal | Up | none | polygon | yes | KEY-O-KEY-P22 |
| Channel Haul | c | none | polygon | yes | KEY-O-KEY-P23 |
| Edge Extend | z | none | edge | yes | KEY-O-KEY-P24 |
| Subdivide | d | none | polygon | yes | KEY-O-KEY-P25 |
| Select Loop | l | none | edge | yes | KEY-O-KEY-P28 |
| Invert Selection | [ | none | polygon | yes | KEY-O-KEY-P29 |
| Select Connected | ] | none | polygon | yes | KEY-O-KEY-P30 |
| Drop Tool | q | none | polygon | yes | KEY-O-KEY-P31 |

**KEY-02 B changes tool with component selection mode.** The same physical
key opens Polygon Bevel, Edge Bevel or Vertex Bevel for Polygon, Edge or
Vertex mode respectively. With the +Z polygon selected, activating Polygon
Bevel already exposes a zero-offset preview with 12 points and 10 polygons;
the selected polygon list remains [1]. The edge and vertex tests retain
8 points and 6 polygons at activation. These counts describe the live
preview, before dropping the tool. (KEY-O-KEY-P07, KEY-O-KEY-P14,
KEY-O-KEY-P16.)

**KEY-03 S opens different Slide controls for an edge and a vertex.** A
selected edge exposes Percent, Loop Slide and Use Handle Vector controls;
a selected vertex exposes Direction X/Y/Z and Merge Vertices controls.
Neither input changes positions or counts at activation. One selected
polygon produces a disabled-command message. (KEY-O-KEY-P13,
KEY-O-KEY-P15, KEY-O-KEY-P17.) This answers KEY-Q02 for B and S only.

**KEY-04 Selection shortcuts act on the current component selection.**
On the cube, L with edge [4,5] selected selects the four edges around the
+Z face. In Polygon mode, [ replaces [1] with [0,2,3,4,5], and ] expands
[1] to all six polygons. The mesh remains eight points and six polygons.
(KEY-O-KEY-P28, KEY-O-KEY-P29, KEY-O-KEY-P30.)

**KEY-05 Q drops Move without changing an untouched fixture.** W followed
by Q removes the Move tool properties and leaves eight points, six polygons
and polygon [1] selected. (KEY-O-KEY-P31.)

## Configuration candidates requiring physical modifier verification

The following names are paraphrased from the authorized shipped key map.
They are candidates, not additional observed requirements. The generic
Control qualifier in that map is deliberately left unresolved on macOS.

| UI action | Key | Modifier candidate | Context | Verified by press |
|---|---|---|---|---|
| Polygon Bevel | B | Shift | global | no (plain B has the same result) |
| Polygon Slice | C | Shift | global | no (Input Editor confirms assignment) |
| Loop Slice | C | Option | global | no |
| Uniform Scale | R | Shift | component | no |
| Extrude | X | Shift | component | no |
| Radial Sweep | L | Shift | component | no |
| Mirror | V | Shift | global | no |
| Flex | Y | Shift | component | no |
| Bend | E | generic Control qualifier; physical modifier unresolved | global | no |
| Soft Drag | T | generic Control qualifier; physical modifier unresolved | global | no |
| Automatic action center | A | Option | global | no |
| Selection action center | S | Option | global | no |
| Selection Center Auto Axis | D | Option | global | no |
| Origin action center | W | Option | global | no |
| Local action center | X | Option | global | no |
| Element action center | Z | Option | global | no |
| Screen action center | F | Option | global | no |
| Pivot action center | E | Option | global | no |
| Pivot Center Parent Axis | G | Option | global | no |
| Convert to Vertex / Edge / Polygon | 1 / 2 / 3 | Option | component | no |
| Select Ring | L | Option | component | no |
| Select Between | G | Shift | component | no |
| Expand / Contract Selection | Up / Down | Shift | component | no |
| Triangulate | T | Shift | component | no |
| Fit Work Plane to Geometry | Home | none | viewport | no |
| Fit Work Plane to Selection | Home | Shift | viewport | no |
| Reset Work Plane | End | none | viewport | no |
| Snapping while held | X | none | global | no (hold/release not tested) |

## Cmd versus Ctrl (KEY-Q03)

Still open. Command+E produced no observed activation; Control+E opened
Rotate, matching plain E rather than the shipped Bend candidate.
Shift+C and Option+C also opened the plain-C action, Channel Haul, although
Shift+C entered uppercase C in an editable text field. These observations
do not establish normal physical modifier behavior. No global conversion
between Command and Control is specified. Modifier delivery, active-map
customization and context routing remain possible explanations.

## Remaining scope

The complete raw input map remains in research evidence. This partial spec
does not yet cover all modeling assignments or every selection-dependent
key, layout context, hold/release binding, primitive shortcut, falloff,
symmetry, modified-key combination or custom-map interaction. Undo is not
tested here. *(open: KEY-Q01, KEY-Q02 remainder, KEY-Q03, KEY-Q05)*
