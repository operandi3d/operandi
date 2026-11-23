<!-- help-ids: Bevel, poly.bevel ; template: tool ; since: 0.1.0 -->
# Bevel (polygons)

**Bevel** pushes the selected polygons out (**Shift**) and shrinks them
in (**Inset**), and builds new side polygons to join them to the rest of
the mesh.

![Bevel on the top of a cube: the cap moved up and in, with the tool's
handles and its Tool Properties](../images/bevel-active.png)

Bevel is a polygon tool. It is on the **Polygon** tool tab, and its key is
**B** in Polygon mode. (In Edge mode **B** starts **Edge Bevel**, and in
Vertex mode it starts **Vertex Bevel**, which have pages of their own.)

## How to use

1. Press **3** to switch to Polygon mode.
2. Click the polygons you want to bevel. With nothing selected, Bevel works
   on every polygon of the mesh.
3. Press **B**, or click **Bevel** on the **Polygon** tool tab.
4. Drag up or down in the **3D view** to change **Shift**.
5. Drag left or right to change **Inset**.
6. Release the mouse button. The bevel is made.
7. Press **Q** to drop the tool when you are done.

Each new drag makes another bevel on top of the last one, starting from the
new cap. To fine-tune the bevel you just made instead, type new values for
**Shift** or **Inset** in **Tool Properties**: the last bevel updates as you
type.

Choosing the tool doesn't change the mesh. Nothing happens until you drag,
type a value or click **Apply**.

## Handles

| Handle | Drag it to |
|---|---|
| Blue arrow, along the polygons' normal | Change **Shift**: how far the cap moves out. |
| Red handle, sideways | Change **Inset**: how far the cap's edges move in. |
| Left drag away from the handles | Up and down changes **Shift**; left and right changes **Inset**. |
| Right drag away from the handles, sideways | Change **Segments** by one every few pixels. |

Rest the pointer on a handle to see what it does.

## Properties

These are in **Tool Properties** while Bevel is active. Distances use your
**Distance Units** preference.

| Name | What it does | Default |
|---|---|---|
| **Shift** | How far the cap moves along the polygons' normal. A negative value pushes it in. | 0 m |
| **+/−** (under Shift) | Adds a random amount, up to this much either way, to each polygon's Shift. With **Group Polygons** on, the whole group gets one random amount. | 0 m |
| **Inset** | How far the cap's edges move in across the polygon. A negative value grows the cap outward. | 0 m |
| **+/−** (under Inset) | The same random spread, for Inset. | 0 m |
| **Edges** | **Inner** or **Outer**: **Outer** moves the cap the opposite way. | Inner |
| **Use Material** | Gives the new side polygons the material named below. | Off |
| **Material Name** | The material for the side polygons. Needs **Use Material**. | (empty) |
| **Group Polygons** | On: polygons that touch are beveled as one piece and share their side walls. Off: each polygon is beveled on its own. | On |
| **Segments** | How many rows of side polygons the bevel has. Whole numbers from 1 to 4096. | 1 |
| **Square Corner** | Keeps corners square where one polygon meets the bevel. | Off |
| **Edge Rail** | Slides the new edges along the existing edges of the group, so the bevel follows them. Needs **Group Polygons**. | Off |
| **Offset Even** | Keeps the inset the same distance from every edge, so corners stay crisp. Off, each corner moves by **Inset** along its own diagonal. | On |
| **Thicken Polygon** | Keeps a reversed copy of the original polygons behind the bevel, so the result is a closed, thick piece. | Off |
| **Max. Smoothing Angle** | Polygons that meet at less than this angle push out together, as one smooth surface; sharper corners push out separately. | 89.9° |
| **Use Vertex Normal Map** | Pushes the cap along the mesh's vertex normal map instead of its polygon normals. | Off |
| **Open Boundary** | Keeps points on an open edge of the mesh on that edge, so an open border doesn't pull in. | Off |
| **Auto Weld** | Joins new points that land on top of each other, for example when **Inset** is bigger than the polygon. | Off |
| **Weld Range** | How close points must be for **Auto Weld** to join them. Needs **Auto Weld**. | 0.1 mm |
| **Falloff Weight** | With a falloff on: **Vertex** weights each point, **Averaged** each polygon by its points' average, **Polygon** each polygon as a whole. Needs **Group Polygons** off. | Averaged |
| **Falloff Shift** | Lets the falloff scale **Shift**. | On |
| **Falloff Inset** | Lets the falloff scale **Inset**. | On |
| **Reverse Direction** | Turns the profile shape the other way. Needs a **Profile**. | Off |
| **Reverse Inset** | Flips the profile shape across the inset. Needs a **Profile**. | Off |
| **Keep Aspect** | Ties **Inset** to **Shift** so the profile keeps its proportions; **Inset** is greyed while it is on. Needs a **Profile**. | On |
| **Make UVs** | How the side polygons get texture coordinates: **Connected**, along **U**, or along **V**. | Connected |
| **Apply** | Makes a bevel with the current values, without dragging. | |
| **Profile** | A shape for the side walls: **None**, **Original Linear** or **Original Smooth**. **Content Browse** lists more profile shapes. | None |
| **Tool State** | Saved sets of Bevel values. Type a **Name** and click **Save** to keep the current values; pick one from the list to use it again. | None |
| **Reset Tool Attributes** | Puts every value back to its default. Also **Ctrl+D** while Bevel is active. | |

## Tips

- Zero **Shift** and zero **Inset** still build new side polygons, flat on
  top of the old edges. That's useful as a start for a second bevel, but it
  leaves overlapping points if you stop there; press **Ctrl+Z** to take it
  back.
- Bevel remembers its values when you drop it, so the next bevel starts
  where the last one left off. Use **Reset Tool Attributes** to start
  fresh.
- Turn **Group Polygons** off to bevel a field of polygons into separate
  studs or panels in one go. A little **+/−** makes them uneven, which
  hides the "made by a computer" look.

## Related

- [Getting started](../getting-started.md): selecting, undo and moving
  around the view.
- [Interface](../interface/window.md): where **Tool Properties** and the
  **Toolbox** are.
- Edge Bevel, Vertex Bevel and Twist Bevel (pages not written yet).
