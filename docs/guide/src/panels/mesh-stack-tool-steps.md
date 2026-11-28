<!-- help-ids: edge.bridge.item, poly.loopSlice.item, poly.thicken.item, poly.setType.meshop.item, edge.slide.item, setvmap.subd.item, hardedge.meshop.item, hardedgeconvert.meshop.item, chamfer.edit.item, pmodel.partTag.item, assignselectionset.meshop.item, curveconvert.meshop.item, curveWinding.meshop.item, curvefill, curveBoolean.meshop.item, pmodel.edgeToCurve.item, cRebuild.mOp, curve.rebuildSpan, laceGeom, uvtransform, deformer.rebind, curve.sweep, radial.sweep.item, prim.text.item, prim.arc.item, uv.create.item, uv.relax.item, uv.pelt.item, uvflip.meshop.item, uvconvert.meshop.item, setvmap.create.item, setvmap.weight.item, setvmap.flood.item, setvmap.rgb.item, setvmap.seam.item, setvmap.vnorm.item, pmodel.geodesic.mOp, Create UV Map, UV Relax, UV Unwrap and Relax, UV Flip, Convert UV to Mesh, Create Vertex Map, Set Weight, Flood Weight, Set RGB, Set Seam, Set Vertex Normals, Geodesic Weight, Curve Convert, Curve Winding Aligner, Curve Fill, Curve Boolean, Curve Rebuild, Curve Rebuild Spans, Lace Geometry, UV Transform, Curve Sweep, Assign Selection Set, Set Polygon Type, Hard Edge, Hard Edge Convert, Part Tag ; template: concept ; since: 0.1.2 -->
# Tool steps in the Mesh Stack

Many modeling tools can also be a step in a mesh's **Mesh Stack**, so you
can change their values later without rebuilding the mesh. A step runs
the same code as its tool, so the tool and the step give the same result
from the same values. Add one from the stack's **Add Operator** list. Its
values show in **Properties**, and its **Selection** section says which
part of the mesh it works on (see
[Mesh Stack selections](mesh-stack-selections.md)).

With nothing in its **Selection**, a step works on the whole mesh: every
polygon, or every edge for the edge steps.

## The steps

| Step | Folder | Works on | What it does |
|---|---|---|---|
| **Bridge** | Edit | Edges | Joins two edge runs or loops with new polygons, as [Bridge](../duplicate/bridge.md). With nothing selected it bridges the mesh's open borders, so two loose pieces with one open border each join up. |
| **Loop Slice** | Edit | Edges | Cuts loops across the ring of each chosen edge, as [Loop Slice](../edit/slice-tools.md). With a **Count** above 1 the loops are spaced evenly and **Position** is set aside. |
| **Hard Edge** | Edit | Edges | Marks the chosen edges hard (or, with **Hardness** set to **Soft**, soft) and rebuilds the vertex normals. **Mode: By Angle** takes only the chosen edges sharper than **Angle**. **Clear Other Hard Edges** softens the rest. |
| **Hard Edge Convert** | Edit | Whole mesh | Turns every edge where the shading splits into a hard edge. It can also remove the normal map and the smoothing groups. |
| **Edge Slide** | Edge | Edges | Slides the chosen edges along their neighboring edges, as [Slide](../edit/slide.md), by **Percent** or by **Distance**. |
| **Chamfer Edit** | Edge | Polygons | Reworks a chamfer or bevel made lower in the stack: pick its strip polygons, then change **Offset**, **Segments** and **Depth**. |
| **Thicken** | Polygon | Polygons | [Smooth Shift](../polygon/smooth-shift.md) with **Thicken** on: a sheet gets a back side and walls. **Offset** sets the thickness. |
| **Set Polygon Type** | Polygon | Polygons | Makes the chosen polygons plain faces, **Subdivision** polygons or **Catmull-Clark** polygons. It sets the type rather than switching it, so it gives the same result every time. |
| **Part Tag** | Polygon | Polygons | Gives the chosen polygons a part name, as [Set Part](../polygon/part-material.md). |
| **Assign Selection Set** | Selection | Polygons | Stores the chosen polygons as a selection set called **Name**, so steps above it can pick them with **Select by Selection Set**. |
| **Set Edge Weight** | Vertex Map | Edges | Gives the chosen edges a subdivision weight, as [Edge Weight](../edge/edge-weight.md). **0%** removes their weights. |
| **Radial Sweep** | Duplicate | Edges | Spins a profile round an axis into a surface, as [Radial Sweep](../duplicate/radial-sweep.md). With nothing selected the whole mesh is the profile, and its lines and curves are used up. |
| **Text** | Create / Primitives | Adds | Adds the letters of **Text** as flat polygons, as the [Text](../modeling/text.md) tool. Empty text adds nothing. |
| **Arc** | Create / Primitives | Adds | Adds an arc-shaped polygon round **Center**, starting toward **Start** and turning **Angle** toward **End**, as the [Arc](../modeling/arc.md) tool. |

## Curve steps

These steps work on the curves in the mesh. Polygons among the chosen
elements pass through unchanged, and with nothing selected a step uses
every curve.

| Step | Works on | What it does |
|---|---|---|
| **Curve Sweep** | Curves | Puts a round tube of diameter **Size** along each curve. **Steps** sets how many rings follow the curve (or **Auto** adds them where it bends), **Rotation** rolls the tube and **End Caps** closes its ends. **Curve Index** picks curves by their order, for example `0,2`. The curves stay. |
| **Curve Fill** | Curves | Fills each closed curve with a flat polygon, as [Make Curve Fill](../curve/curve-commands.md). **Delete Source Curve** (on at first) removes the curves; **Flip Source Curves** turns the fills over. |
| **Curve Boolean** | Curves | Cuts the chosen closed curves against the others: **Slice**, **Union**, **Intersect** or **Subtract**. With nothing selected the curves cut each other. |
| **Edges to Curves** | Edges | Turns the chosen edges into curves, joined end to end (**Continuous**) or one per edge (**Separate**), as lines, Curves or Beziers. With nothing selected it traces the open borders. **Delete Geometry** removes the polygons the edges came from. |
| **Curve Rebuild** | Curves | Rebuilds each curve through evenly spaced points: **Point Count** points, or one every **Point Spacing**. |
| **Curve Rebuild Spans** | Edges | Splits each chosen span of a curve into **Count** spans, or one every **Distance**, keeping its shape; the curve comes out as a Bezier. **Offset Amount** pushes the new knots (all knots with **Effect All Points**) along the **Offset Vector**, the curve's own bend (**Curve Normal**) or the **Alignment Mesh**'s surface (**Mesh Normal**), by **Apply**. **Base Position: Mesh Position** first snaps the knots onto the Alignment Mesh, found by **Normal Method** within **Max Search Distance** (0 is no limit). **Falloff By Parameter** fades the push toward the original knots. |
| **Lace Geometry** | Curves of another mesh | Adds a capped tube along curve **Guide Curve ID** (counting from 0) of the **Guide Mesh**, from **Start** to **End** of its length. **Shape** is Circle, Oval, Square or Rectangle (**Sides**, **Aspect Ratio**, **Radius**, **Shape Angle**); rings follow bends wherever the curve turns more than **Refinement Angle**. **Make UVs** writes the Texture map, repeating every **Pattern Size** along the tube. The mesh's own geometry stays. |
| **Curve Convert** | Curves | Converts curves of one type (**From**) to another (**To**): Curve, Bezier or B-Spline, as [Convert Curves](../curve/offset-convert.md). It starts as Curve to Bezier. |
| **Curve Winding Aligner** | Curves | Makes every closed curve run the same way round, as [Align Winding](../curve/curve-commands.md). **Merge Curves** first joins open curves that meet end to end; **Reverse** turns them all the other way. |

## UV and vertex map steps

These steps write into a mesh's UV maps and vertex maps. A step names its
map in **UV Map** or **Map Name**; a map that doesn't exist yet is made.

| Step | Works on | What it does |
|---|---|---|
| **Create UV Map** | Polygons | Projects the chosen polygons into the map: Planar, Cylindrical, Spherical, Atlas, Barycentric, Atlas 2 or Cubic. **Settings: Automatic** fits the shape to the polygons; **Manual** uses **Center**, **Size** and **Rotation**. |
| **UV Relax** | Polygons | Relaxes the chosen polygons' UVs, as the UV Relax tool. **Iterations** starts at 0, which leaves the UVs alone, so raise it to see a change. Polygons not chosen stay still. |
| **UV Unwrap and Relax** | Polygons | A UV Unwrap followed by a UV Relax, each switched on or off with its own checkbox and with its own **Iterations**. |
| **UV Flip** | Polygons | Mirrors the chosen polygons' UVs in U or V about their middle, as [Flip UVs](../uv/flip-uvs.md). **Flip per Polygon** flips each polygon on its own. |
| **Convert UV to Mesh** | Polygons | Replaces the mesh with its flat UV layout: U along X, V along Y. |
| **Create Vertex Map** | Whole mesh | Adds an empty map called **Map Name**: Weight, UV, Morph, RGB or RGBA. |
| **Set Weight** | Points | Gives the chosen points **Map Value** in a weight map, or adds it with **Additive**. |
| **Flood Weight** | Points | Sets every chosen point to 100% (**High**) or 0% (**Low**). |
| **Set RGB** | Points | Gives the chosen points a colour in a colour map; **Map Type: RGBA** uses **Value A** too. |
| **Geodesic Weight** | Points | Makes a weight map of each point's distance from the chosen points, measured along the edges. **Normalize** makes the farthest point 100%; **Invert** turns it round. |
| **Set Seam** | Edges | Adds the chosen edges to a UV seam map, so a UV Unwrap above can cut there. |
| **Set Vertex Normals** | Points | Rebuilds the vertex normal map on the chosen points, splitting at hard edges. |

## Slice steps

These steps cut new edges into polygons along a path, as the Slice tools
do: Primitive Slice, Curve Slice and Pen Slice (see
[Slice tools](../edit/slice-tools.md)). The path is pushed along **Axis** (X, Y, Z, **Custom** with
**Vector X/Y/Z**, or **Action Axis**) and cuts every polygon it passes,
or only the step's chosen polygons. No point moves.

| Step | What it cuts with |
|---|---|
| **Primitive Slice** | A **Rectangle**, **Circle** or **Ellipse** at **Center**, lying square to its own **Axis X/Y/Z** (Action Axis cuts along that axis). **Half Size**, **Radius**, **Sides**, **Corner Radius**, **Heading**, **Pitch**, **Bank**, **Flip** and the **Clone X/Y** grid work as in the tool. |
| **Curve Slice** | One of the mesh's own curves, picked by **Curve Index** (0 is the first). **Steps** sets how many straight pieces the curve is cut in (or **Automatic**, by **Smoothing Angle**). **Start** and **End** keep part of the curve and **Slide** moves that part along it. The curve itself isn't cut and stays. |
| **Pen Slice** | One of the mesh's curves, picked by **Polyline Index**, as straight lines between its points. A closed curve cuts a closed outline. |

**Operation** picks what the cut does: **Slice** only cuts; **Stencil**
gives the polygons inside a closed outline **Material**; **Core** keeps
only those inside polygons; **Tunnel** deletes them, leaving a hole. **Select
Sliced Edges** adds the new edges to the edge selection set named in
**Edge Selection Set**, so a step above can use them. With no curves in
the mesh, Curve Slice and Pen Slice pass the mesh through.

## Tool pipe steps

These steps don't change the mesh themselves. Each one sets something for
every step above it in the stack, until another step of the same kind
replaces it. Turn a step's eye off and it sets nothing.

| Step | What it does to the steps above |
|---|---|
| **Symmetry** | A step with a **Selection** also works on the mirror image of what it picked, across the mesh's own X, Y or Z = 0 plane (**Axis**). Elements with no mirror partner are used as they are. |
| **Set Work Plane** | Primitives and curve shapes are made on this plane: moved by **Offset X/Y/Z** and turned by **Angle X**, then **Y**, then **Z**, about the mesh's origin. Geometry that's already there doesn't move. |
| **Falloff** | Steps that move points move each point only by the falloff's strength there: fully where it's 100%, not at all where it's 0%. The step keeps its own copy of the active falloff from when you add it; **Use Active Falloff** and **Clear Falloff** in its properties change it. Primitives above it (Cube, Sphere, Cylinder, Cone, Capsule, Ellipsoid, Torus) are built with it through their own **Falloff** rows. Other steps that add or remove points aren't shaped, and their row says so. Select the step to drag its falloff's points in the view. A Falloff step with no falloff turns off the one below it. |

## Deformer steps

Deformers live in their own **Deformers** folder when you add a step. They
only move points: they never add or remove any, so the mesh's polygons stay
the same. Each one moves the points its **Selection** picked (every point
with no Selection) by its **Strength**: 100% moves them fully, 50% half
way. Put a **Falloff** step below a deformer to fade it out across the mesh.

With no curve to follow, the curve deformers pass the mesh through.

| Step | What it does |
|---|---|
| **Bend** | Curls the points into an arc. Points further along the **Spine** (from **Center**) turn further, up to the full **Angle** at the spine's end, around **Axis**. Points beyond the end turn with it; points behind the center stay, or mirror with **Both Sides**. |
| **Twist** | Turns the points around a line through **Center** along **Axis**. Points at the center don't turn; points at **Length** along the axis, and beyond, turn by the full **Angle**. |
| **Transform Effector** | Moves, turns and scales the points about **Center**: scaled by **Scale X/Y/Z** first, then turned by **Rotation X**, then **Y**, then **Z**, then moved by **Position X/Y/Z**. A new step changes nothing. |
| **Rotate Effector** | Turns the points by **Angle** around a line through **Center** along **Axis**. |
| **Magnet** | Pulls the points near **Center** by **Offset X/Y/Z**: fully at the center, fading smoothly to nothing at **Radius**. |
| **Weight Influence** | Moves each point by **Offset X/Y/Z** times its value in the weight map named in **Map Name**, so painting the map paints the move. Points the map doesn't cover stay. |
| **Push Influence** | Pushes each point out along its normal by **Distance**. With a **Map Name**, each point is pushed by its share of that weight map. |
| **Morph Influence** | Moves each point by its offset in the morph map named in **Map Name**, times **Strength**, so you can dial a morph in partway, or past 100%. |
| **Vortex** | Swirls the points around a line through **Center** along **Axis**: points on the line turn by the full **Angle**, and the turn fades smoothly to nothing at **Radius** from it. |
| **Lattice** | A box cage from **Min** to **Max** with **Divisions X/Y/Z** control points. Select the step and drag a control point in the view: the points inside the box bend smoothly with it. Points outside the box don't move. Changing **Divisions** starts the control points over. |
| **Spline Deformer** | Bends the mesh along one of its own curves (**Curve Index** counts them from 0). The distance along **Axis**, from **Start** and times **Length Scale**, becomes the distance along the curve, so a straight column follows the curve. Past the curve's ends the mesh carries on straight. The curve itself doesn't move. |
| **Bezier Deformer** | The same bend as **Spline Deformer**, along one of the mesh's Bezier curves (**Curve Index** counts only those). |
| **Curve Constraint** | Pulls each point onto the nearest spot of one of the mesh's curves, by **Strength**. With a **Radius**, only points that close to the curve are pulled; 0 pulls every point. |
| **UV Transform** | Moves each point onto the **Target** mesh's surface, using the Target's **Target UV Map** as the address: the point's place seen down **Axis**, in a square of **Scale** (100% is 1 m) centred on **Offset**, is a UV spot, and the point goes where the Target has that UV. With a **Source UV Map**, the point's own UV is the address. **Wrap** off keeps each point's depth off the surface; **Push** lifts every point along the surface's normal. **Use Source Transform** reads the places where the mesh sits in the scene. Points whose address is on no Target polygon stay. It moves points only; it doesn't change UVs. |
| **Wrap** | Sticks each point to its nearest spot on the **Target** mesh, so the mesh follows when the target moves or changes shape. Pick a **Target**, then press **Rebind** in the step's **Binding** section; the binding is saved with the scene. Press **Rebind** again after changing the mesh below or the target's polygons (the row tells you when). **Strength** below 100% moves the points only part of the way. Scripts rebind with `deformer.rebind`. |

A step that can't run (for example **Bridge** on a closed mesh with no
open border) turns its row red and passes the mesh below through
unchanged, so the rest of the stack still works.

## Related

- [Mesh Stack selections](mesh-stack-selections.md)
- [Properties panel](properties.md)
