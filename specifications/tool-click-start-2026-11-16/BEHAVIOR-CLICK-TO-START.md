# Click to start: the shared first-click layer (2026-11-16)

Behaviour spec, Batch B. Bend's own first-click rules (spine fit BN-24,
BN-30a, BN-30b) live in `specifications/bend-handles-2026-11-15/`.

## 1. Auto-Activate

- **CS-01** Every tool on the pipe has one flag, Auto-Activate.
  - On: picking the tool makes it live at once and shows its handles.
  - Off: picking the tool only ARMS it. The Tool Properties panel shows the
    values and they can be edited. Apply runs the tool once with those
    values and leaves it armed (CS-21). Nothing is drawn in the view and
    the mesh doesn't change. Dropping an armed tool that was never clicked
    changes nothing (CS-22). The first left click in a 3D view makes it
    live.
- **CS-05** The flag is a column in the tool-presets table. A user toggle
  (Tool Pipe panel) can come later; no UI is needed now.
- **CS-06** Status bar while armed: "Click in a view to start <tool>, or
  set values and press Apply."

## 2. Which tools wait for a click

Auto-Activate OFF (waits for a click): Bend, Taper, Shear, Push, Flare,
Soft Drag, Element Move, Element Push, Jitter, Smooth, Quantize, Thicken,
Polygon Extrude, Smooth Shift, Edge Extend, Edge Knife/Edge Slice, Polygon
Knife, Bridge, all primitives, Pen, Mirror, Array, Radial Array, Clone and
the clone family, Radial Sweep, Curve Slice, Snap Drag, Pen Knife, Drag
Weld, Magnet, Pole (Bulge), and any of these Operandi has.

Auto-Activate ON (starts at once, unchanged): Move/Rotate/Scale/Transform
(and Item versions), Twist, Swirl, Vortex, Flex, Soft Selection tools, the
Slides, Polygon/Edge/Vertex Bevel, Edge Chamfer, Vertex Extrude, Loop
Slice, Ruler, Protractor, Solid Sketch, guides.

## 3. The first click

- **CS-02** The first click runs through the pipe, in order:
  - **CS-10** Automatic action centre: the centre goes to the click (mesh
    surface, else the work plane; snapping applied).
  - **CS-11** Auto action axis: set to the work-plane normal.
  - **CS-12** Other action centres (Selection, Element, Origin, Pivot): the
    click doesn't move the centre (Element: it picks the element, as
    today).
  - **CS-13** Self-fitting falloffs (Linear on Taper/Shear, Radial) fit to
    the target on that click.
  - **CS-14** The tool's own first-click setup. Bend: spine fit.
    Primitives: start drawing. Array/Clone: the first offset is the
    selection box size. Radial Array/Sweep: the click is the rotation
    centre. Pen: the click is the first point. Others: the click just shows
    the handles.
  - Then the handles show.
- **CS-15** The activating press can continue as a drag. Primitives draw.
  For other tools the drag after the activating press is the tool's normal
  off-handle drag (provisional, CS-Q04).
- **CS-20** A later click away from the handles re-runs the centre and axis
  rules plus the tool's own click setup (Bend refits every time).
- Existing private versions fold into this layer: the Duplicate tools'
  "nothing until first click or Apply" (DS-30/31) and the numeric primitive
  editors.

## 4. Provisional (CS-Q01..Q04)

Logged in `docs/SPECIFICATION_QUESTIONS.md`.
