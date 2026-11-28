<!-- help-ids: Tube, prim.tube ; template: tool ; since: 0.1.0 -->
# Tube

**Tube** adds a round tube that follows a smooth path through points you click.

![A tube through four points clicked on the ground, with its Tool Properties](../images/tube.png)

## How to use

1. Make sure the mesh you want to add to is selected in the **Items list**.
2. Click **Tube** in the **Draw** section of the **Create** tool tab.
3. Click in the **3D view** to add points on the **Work Plane**. The tube
   appears once there are two.
4. Drag a point to move it, or click it and type its place in
   **Tool Properties**. Middle-click a point to delete it
   (**Shift**+middle-click deletes it and every later one).
5. Click **Drop** to keep the tube, or **Apply** to keep it and start
   another.

**Mode** at the top of **Tool Properties** picks what a click does:
**Add** points, **Edit** them, or **Delete** them. **Add Point** and
**Remove Last** add or remove a point at the end of the path.

## Properties

| Name | What it does | Default |
|---|---|---|
| **Position** | The current point's place (X, Y, Z): the point you last clicked, else the last one. | the last point |
| **Sides** | How many polygons go round the tube. | 12 |
| **Segments** | How many rows of polygons go between two points. | 10 |
| **Radius** | The tube's radius. | 0.2 m |
| **Twist** | Turns the rows progressively along the tube. | 0° |
| **Caps** | Closes each open end. Greyed when **Closed** is on. | On |
| **Closed** | Joins the last point back to the first. | Off |
| **By Length** | Spaces the rows evenly along the whole tube instead of evenly between points. | Off |
| **Make UVs** | Gives the shape a UV layout in the **Texture** UV map, for textures. | On |

- **Apply** adds the tube to the mesh and keeps the tool, so you can
  make another.
- **Drop** adds it and closes the tool.
- **Cancel** throws the preview away.
- **Reset** puts every value back to its starting value.

## Tips

- Right-drag in the **3D view**, away from the handles: left and right change **Sides**, up and down change **Segments**.

## Related

- [Primitives](../interface/window.md): the **Toolbox** on the **Create** tab
- [Interface](../interface/window.md): the **Work Plane** and **Tool Properties**
- [Cylinder](cylinder.md)
