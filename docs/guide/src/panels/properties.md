<!-- help-ids: Properties ; template: panel ; since: 0.1.0 -->
# Properties panel

The **Properties** panel shows the settings of the selected item. With
an item selected, tabs down its right side switch between pages of
settings.

![The Properties panel for a selected mesh item](../images/panel-properties.png)

## Pages (for a mesh item)

| Tab | Holds |
|---|---|
| **Mesh** | The item's name, **Item Transform** (position, rotation and scale, with **Reset** and **Freeze**) and **Subdivision**. |
| **Surface** | Smoothing and other surface settings. |
| **Display** | How the item draws in the 3D view. |
| **Assembly** | Settings for the item's place in an assembly. |
| **User Channels** | Channels you add yourself. |
| **Tags** | Tags on the item. |

Other kinds of item (cameras, lights, the render item) show their own
pages. A shader tree layer's settings show here when you click it in the
[Shader Tree](shader-tree.md).

## Common tasks

- **Type an exact value**: click a field, type, and press **Return**.
  Fields take expressions, such as `250 mm` or `1 m/4`. Nothing changes
  until you press **Return** or **Tab**, or click somewhere else; **Esc**
  puts the old value back.
- **Link X, Y and Z**: click the chain at the end of the first row, so
  changing one changes all three.
- **Animate a value**: click the dot before a field to set a key on it.
- **Copy values to another item**: click a row's label to select that
  value; **Shift**-click more labels to add them. Right-click one of them
  and choose **Copy**. Select the other item, select the same rows
  there, right-click one of them and choose **Paste**. Each row takes the
  copied value of the same name. One **Undo** takes it back.
- **Freeze** the transform into the mesh, or **Reset** it to zero, with the
  two drop-downs under **Item Transform**.

## Related

- [Items list](items-list.md)
- [Interface](../interface/window.md): part 12 is the Properties panel
