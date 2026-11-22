# Data-driven Tool Properties: GUI checklist

Branch `tool-props-data`. Tool Properties now draws from each tool's field
list (`crates/operandi-app/src/tool_form.rs`). The panels should look and
behave as before; these checks look for anything that moved.

## Pilot (Cube, Sphere, Cylinder, Cone, Ellipsoid, Torus, Capsule, Arc, Tube, Text, Spike, Polygon Reduction)

For each tool, start it from the Create tab (Spike and Polygon Reduction
from their menus, on a mesh with polygons):

1. The rows are in the same order with the same labels, units and
   greyed rows as on `main`.
2. Type a value and press Enter: the preview updates and the field shows
   the value in its unit (`1/2` shows `0.5 m`, FORM-Q14).
3. Type nonsense and press Enter: the field gets a red outline and the
   status bar says why; Escape puts the last value back.
4. Drag a number field's arrows: the preview follows; one Undo takes back
   the whole drag. A click on a whole-number field's arrows steps it by 1.
5. Tab / Shift+Tab move between the number fields (UIC-51).
6. Change a checkbox, dropdown or segment: the preview updates at once.
   Type in a field without pressing Enter, then change an option: both
   changes land.
7. Apply / Drop / Cancel / Reset do what they did before.

Tool specifics:

- Sphere: Sphere Mode Globe greys Subdivision Level; Tessellated enables
  it; Quad Ball is listed greyed. Subdivision Level now has arrows (FORM-Q15).
- Cylinder: Cap Top and Cap Bottom dropdowns change each end.
- Cone: Top Cap lists Radial greyed; Edge Weight % is greyed for Face.
- Tube: Add Point / Remove Last add and remove a row of Point fields.
- Text: typing in Text and Enter updates the preview; Font is a dropdown
  row lined up with the others (FORM-Q16).
- Polygon Reduction: after the first click on the mesh, Number shows the
  mesh's count without touching the field.

## Every other tool (branch `tool-props-data-2`)

Run the seven pilot checks above on each of these. Live tools need their
selection (Edge tools: an edge between two polygons; Vertex tools: a
point; Chamfer Edit and Edge Relax: a bevelled strip).

- Transform, Move, Rotate, Scale; Move/Rotate/Scale from a Deform preset
  (falloff rows under the transform: Start/End or Center/Size, Shape
  Preset, Reverse, Fit to Selection). Scale/Transform in Item mode show
  Negative Scale; Move/Rotate/Transform on a camera or light show its
  channels (Focal Length in mm, Cone Angle in °).
- Tweak (Range greys with Rigid), Push (type Distance: it now moves the
  points, FORM-Q20), Smooth (Iterations, Sharp Angle greys without Lock
  Sharp Edges).
- Bevel (Inset greys with a profile and Keep Aspect; Edge Rail with Group
  Polygons; Weld Range with Auto Weld; Profiles / Content Browse / Tool
  States sections; Reset Tool Attributes), Extrude (Make UVs segments grey
  without a UV map, FORM-Q19; UV Map dropdown; Apply; Continue on Caps).
- Inset, Shift, Poly Haul (Mode change), Chamfer Edit, Sketch Extrude.
- Edge Extend, Crease (Additive), Slide (Interpolation resets the amount),
  Edge Bevel and Edge Chamfer (greyed Sharp … Boundary Extend rows stay
  greyed and visible; Chamfer's Edge Shape has no Sharp), Edge Extrude,
  Relax.
- Vertex Create (Position after the first click), Bevel, Extrude, Merge,
  Tear, Slide (Percent ±100 %, Tension greys without Preserve Curvature).
- Loop / Edge / Polygon / Curve / Pen / Primitive / Axis Slice: options,
  Uniform and Remove Last Point buttons, Length in px.
- Bridge (Apply greys until the selection can bridge), Pen (Spline Patch
  greyed in Type; Wall Mode on sets Offset 0.1 m), Smooth Shift / Thicken
  (Tool States: save, recall, rename, delete; Clone).
- Add Loop / Add Point, Paste Tool / Ruler / Protractor / Dimensions /
  Ground Align / Quick Align, Curve / B-Spline / Bezier / Sketch / Draw
  Patch: rows now on the shared label column (FORM-Q22).
- Material Name fields take effect on Enter or leaving the field
  (FORM-Q18).

## Tool list / Tool Properties divider

- A thin line splits the left panel: tool list on top, Tool Properties
  below; each scrolls on its own when its content doesn't fit.
- Hover the line: vertical resize cursor. Drag it up and down; neither
  area gets shorter than about a few rows. Works the same on every tab,
  Select included.
- With Tool Properties under half the panel, activate a tool (e.g. Bevel):
  the divider jumps up so Tool Properties gets half. Drag it lower, switch
  to another tool: it goes back up to half. Drag it above half and switch
  tools: it stays. Drop the tool: it doesn't move.
- Fold the Tool Properties header: the tool list takes the panel, the
  header sits at the bottom and the line goes away. Unfold: the old split
  comes back.
- Quit and reopen: the divider is where you left it (saved in layout.json).
