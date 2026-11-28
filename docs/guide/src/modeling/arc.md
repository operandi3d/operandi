<!-- help-ids: Arc, prim.arc ; template: tool ; since: 0.1.0 -->
# Arc

**Arc** adds one flat polygon shaped like a slice of pie: an arc of a circle, closed at its centre.

![An arc drawn with one drag on the ground, with its Tool Properties](../images/arc.png)

## How to use

1. Make sure the mesh you want to add to is selected in the **Items list**.
2. Click **Arc** in the **Draw** section of the **Create** tool tab.
3. Drag in the **3D view** to draw the arc on the **Work Plane**.
4. Drag one of the arrows at its centre to move it along just X, Y or Z.
5. Type exact values in **Tool Properties** if you need them.
6. Click **Drop** to keep it, or **Apply** to keep it and draw another.

## Properties

| Name | What it does | Default |
|---|---|---|
| **Center** | The centre of the circle (X, Y, Z). | 0 m |
| **Start** | Where the arc starts (X, Y, Z). | 1, 0, 0 m |
| **End** | Which way the arc sweeps; it ends on this side. | 0, 1, 0 m |
| **Radius** | The circle's radius. | 1 m |
| **Angle** | How far the arc sweeps, up to 180°. | 90° |
| **Segments** | How many steps the curved edge has. | 15 |
| **Reverse** | Makes the rest of the circle instead (360° minus **Angle**). | Off |

- **Apply** adds the arc to the mesh and keeps the tool, so you can
  make another.
- **Drop** adds it and closes the tool.
- **Cancel** throws the preview away.
- **Reset** puts every value back to its starting value.

## Tips

- Right-drag in the **3D view**, away from the handles: left and right change **Segments**.
- **Angle** stops at 180°. For more, turn on **Reverse**.

## Related

- [Primitives](../interface/window.md): the **Toolbox** on the **Create** tab
- [Interface](../interface/window.md): the **Work Plane** and **Tool Properties**
