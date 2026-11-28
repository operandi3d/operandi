<!-- help-ids: Action Center ; template: concept ; since: 0.1.0 -->
# Action Centers

An Action Center decides two things for **Move**, **Rotate**, **Scale** and **Transform**: where the handle sits (the center the tool turns and scales about) and which way its three arrows point (the axes your values are measured along). You choose one from the **Action Center** button in the row of modeling aids under the viewport.

The same choice also steers other tools that work about a center, such as **Flex** and **Smooth Shift**.

## Choose an Action Center

1. Click **Action Center** under the viewport.
2. Click a name in the list. The handle of the tool you are using moves and turns at once, and what you have already dragged is kept.

The top line of the menu shows the center and the axis in use. The **Center** and **Axis** submenus at the bottom change one of the two and keep the other.

| Name | Center | Axes |
| --- | --- | --- |
| **None** | Middle of the box around what the tool moves, until you press in the view. | World |
| **Automatic** | Middle of the box around what the tool moves, until you press in the view. | World |
| **Selection Border** | Middle of the selection's border (see below). | Follow the selection's shape |
| **Selection Center Auto Axis** | Middle of the box around the selection. | World |
| **Element** | The polygon, edge or point you press. | From what you pressed |
| **Screen** | Where you press in the view. | Face the view |
| **Selection** | Middle of the box around the selection. | Follow the selection's shape |
| **Origin** | The world origin. | World |
| **Local** | Each connected piece about its own middle. | Each piece's own |
| **Parent** | The parent item's position. | The parent item's |
| **Pivot** | The item's own position. | The item's own, turned with the item |
| **Pivot Center Parent Axis** | The item's own position. | The parent item's |

With nothing selected, "what the tool moves" is the whole mesh.

## Mix a center and an axis

The **Center** and **Axis** submenus let you keep one half of an Action Center and change the other. The top line of the menu always shows both.

**Local** is the one that needs a word of explanation, because it works on each connected piece (each *cluster*) of your selection separately:

- **Local** center with another axis, for example **Local** and then **Axis > World**: each cluster still turns and scales about its own middle, but all of them use that one set of axes. Handy for scaling many separate panels flat along the world Y direction, each in place.
- Another center with **Local** axes, for example **Local** and then **Center > Origin**: everything turns and scales about that one center, with the axes of the cluster you selected last. When the selection was made in one go, with **Select All** or by a script, the last polygon, edge or point in its list counts as selected last, so the same selection always gives the same axes.

The handle always shows the center and axes your values are measured in. Under **Local** it sits on the cluster you selected last.

## Local

**Local** gives every cluster of your selection its own center and its own axes, so one drag or one typed value works on each of them in place. Each cluster's axes follow its surface:

- **Z** (blue) points straight out of the cluster, along its polygons' normal. On a cluster that curves, it is the direction the cluster is thinnest in.
- **X** (red) lies along the surface, in the direction the cluster is longer or shorter in, whichever is nearer the item's X axis. It always points toward +X.
- **Y** (green) lies along the surface too, square to the other two.

So on a floor, blue is up and red runs along X. On a wall that faces you, blue points at you, red runs to the right and green runs up.

Two cases have no direction nearer X, and get a fixed choice so the handle never turns to a random angle. A square cluster, which is as long one way as the other, takes the item's X laid flat on its surface. A wall that faces straight along X takes its level direction, along the item's Z.

Because red points the same way on every cluster, it is the ring to reach for when you want separate clusters to turn the same way. Select two opposite walls of a box, choose **Local**, and drag the red ring of **Rotate**: both walls lean the same way and the box shears. The green and blue rings turn opposite walls as mirror images of each other, because each wall's blue axis points out of the box on its own side.

Moving along blue pushes each cluster in or out along its own normal. Scaling along blue flattens or thickens it.

Edges in a straight line and lone points are special: a line turns the handle's nearest axis onto itself, and a single point with no polygons keeps the handle's axes as they were.

## Selection and Selection Border

Both give the handle axes that follow the shape of what you selected, so the arrows line up with your model instead of with the world:

- **X** (red) points along the direction the selection is longest in.
- **Z** (blue) points through the direction it is thinnest in. For a flat patch of polygons that is straight out of the surface.
- **Y** (green) is the direction left over, square to the other two.

A few examples: a flat patch gets Z along its normal and X along its longer side. A long strip gets X along the strip. A band of polygons around an arm or a pipe gets X along the arm when the band is longer than the arm is thick.

When two directions are equally long (a square patch, a round ring), the tie is broken toward the world X axis, so the handle does not spin to a random angle. A shape with no longer or shorter direction at all, such as a cube, a single point or nothing to measure, gets the World axes.

The two differ only in where the handle sits:

- **Selection** puts it in the middle of the box around the selection.
- **Selection Border** puts it in the middle of the border: the edges where your selected polygons meet polygons you did not select. Open edges at the rim of a mesh do not count. If the selection touches the rest of the mesh in two places, like a band around an arm, the handle sits halfway between the two. This is the center to use for bending a limb at a joint or turning a panel about its hinge.

**Selection Border** needs selected polygons with unselected neighbors. With edges or points selected, or when you select a whole mesh, it works like **Selection**.

Both work on any selection, whatever its shape.

## With Symmetry on

When **Symmetry** is on and your selection has a mirrored half, the **+** side is in charge:

- The handle sits on the **+** side, and for **Selection** and **Selection Border** its center and its axes come from the selected points on that side only. The mirrored half does not pull the arrows off line.
- The **+** side moves, turns and scales exactly as you drag or type.
- The other side does the mirror image: a move toward the mirror plane becomes a move toward it from the other side, and a turn goes the other way round.

If nothing on the **+** side is selected, the handle goes to the other side and **Selection** and **Selection Border** use the World axes.

**Origin**, **Parent** and **Pivot** keep their fixed centers when Symmetry is on.

## Press in the view to move the center

With **Move**, **Rotate**, **Scale** or **Transform** on, a press in the view that is not on the tool's handle does one of three things, depending on the Action Center. The same press then drags the tool, so you can press and drag in one go. A press on a handle always works the handle.

| Action Center | What a press does |
| --- | --- |
| **None**, **Automatic** | Puts the center where you pressed. Every press moves it again. |
| **Screen** | Puts the center where you pressed and turns the axes to face the view. |
| **Element** | Picks the polygon, edge or point under the pointer. |
| All the others | Nothing: the center stays where its rule puts it. |

What you did before the press is kept.

### None and Automatic

The press lands on the view's work plane, the plane its grid is drawn on: usually the ground in a **Perspective** view, and the facing plane in **Front**, **Top** and **Right**. The center takes its two coordinates in that plane from where you pressed, and keeps its own height above the plane. So over the ground the handle slides under your pointer without rising or sinking.

With **Snapping** on, the center snaps to what is under the pointer: a point, an edge, a polygon or the grid, whichever your snapping settings allow. It can snap to the selection itself, so you can put a pivot on a corner of the piece you are about to turn.

### Screen

**Screen** is in force as soon as you choose it: the center starts in the middle of the box around what the tool moves, and the axes face the view as it is at that moment (X to the right, Y up, Z toward you). Each press places the center the same way **None** does, but never snaps, and takes the view's axes again. Turning the view afterward does not turn the axes.

## Element

**Element** puts the handle on a polygon, an edge or a point of your model and lines it up with it.

1. Choose **Element** from the **Action Center** menu.
2. Turn on **Move**, **Rotate**, **Scale** or **Transform**.
3. Press a polygon, an edge or a point. The handle jumps to it: a polygon's middle, an edge's midpoint, or the point.
4. Press another one to move the handle there. You can do this as often as you like.

It works in every selection mode, on any mesh you can see, and on parts that are not selected. Your selection does not change. A press on empty space leaves the handle where it is.

The axes come from what you picked:

- **Z** (blue) points along its normal. An edge or a point uses the polygons that touch it.
- **X** (red) runs along an edge you picked. For a polygon or a point it is the world X direction, laid flat against the surface.
- **Y** (green) is the direction left over.

Until you pick something, the handle is at the world origin with the World axes. A point or an edge with no polygons has no normal, so it gets the World axes too.

## Pivot

**Pivot** uses the item's own position and the item's own axes. If the item has been rotated, the arrows are rotated with it, so **Move** X slides the selection along the item's own X direction.

## Related

- [Interface](window.md)
- [Tool Properties](../panels/tool-properties.md)
- [Flex](../deform/flex.md)
