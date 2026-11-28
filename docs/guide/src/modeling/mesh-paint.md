<!-- help-ids: Mesh Paint, Mesh Paint Instance, Mesh Paint Instancer ; template: tool ; since: 0.1.0 -->
# Mesh Paint

**Mesh Paint** paints copies of another mesh onto the surface of the selected mesh: pebbles on the ground, rivets on a panel.

![One Slide stroke on a slab put a copy of the small background cube on it](../images/mesh-paint.png)

## How to use

1. Select the mesh to paint onto in the **Items list**.
2. Keep the mesh to copy visible but not selected: it is the *background*
   mesh.
3. Click **Mesh Paint** in the **Measure** section of the **Create** tool tab.
4. Click, or drag, on the selected mesh. Copies of the background mesh land
   on its surface.
5. Press **Q**, or click **Drop**, to close the tool.

**Alt**-click **Mesh Paint** for **Mesh Paint Instance**, which places
instance items instead of copying geometry.

The button stays lit while the tool is on. Mesh Paint keeps the settings
you last used, so turning it off and on again carries on where you left
off.

## Properties

| Name | What it does | Default |
|---|---|---|
| **Paint Mode** | How copies are placed: **Slide**, **Strokes**, **Drop**, **Drop (no Scale)**, **Linear**, **Array** or **Radial**. | Slide |
| **Brush Size** | The brush's radius on screen. | 20 px |
| **Brush Density** | How close together the copies are. | 50% |
| **Size Mode** | How each copy is sized: **Uniform**, **Adaptive**, **Random** or **Locked**. | Locked |
| **Size** | The copies' size along X, Y and Z. | 100% |
| **Scale Increment** | Steps the copies' size snaps to; 0 is free. | 0% |
| **Rotation Mode** | **Uniform**, **Surface Align** (stand on the surface) or **Random**. | Surface Align |
| **Rotation** | Extra turn round X, Y and Z. | 0° |
| **Rotation Snapping**, **Angle** | Snaps the turn to steps of **Angle**. | Off, 15° |
| **Offset** | Lifts the copies off the surface. | 0 m |
| Parameter rows | Let **Falloff**, **Pressure**, **Slope** or **Altitude** change the copies' **Density**, **Size** or **Rotation**. | Falloff and Pressure → Density |
| **Linear Count**, **Array Count X/Y**, **Radial Count** | How many copies the **Linear**, **Array** and **Radial** modes place. | 2; 2 × 2; 6 |
| **Geometry Source** | With several background meshes: **All BG Layers** or **Random BG Layer**. | Random BG Layer |
| **Create Instances** | Each copy is a new instance item of its background mesh instead of geometry added to the selected mesh. | Off |
| **Create Instancer** | Each stroke makes one **Instancer** item, with a point per copy, instead of separate copies. | Off |
| **Parent** | Where new instance items go: **Off**, **Hierarchy**, **Root** or **Primary**. | Off |

## Tips

- Each copy is placed by the background mesh's item origin. Put that
  origin where the copy should touch the surface (at the bottom of a
  pebble, say); geometry far from its origin lands just as far from the
  stroke.

## Related

- [Selection modes](../selection/selection-modes.md)
