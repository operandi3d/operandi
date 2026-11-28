<!-- help-ids: Vertices, Edges, Polygons, Items, select.nextMode, select.drop ; template: concept ; since: 0.1.0 -->
# Selection modes

Most tools work on what is selected. The selection mode says what a click picks: points, edges, polygons or whole items.

![One polygon of a sphere selected with a click](../images/select-click.png)

## The modes

| Mode | Key | A click picks |
|---|---|---|
| **Vertices** | **1** | a point |
| **Edges** | **2** | an edge |
| **Polygons** | **3** | a polygon |
| **Materials** | **4** | every polygon with the same tag (material, part, smoothing group or selection set; pick which in the **Select** tab) |
| **Items** | **5** | a whole item |

The buttons are at the left of the **Mode bar**. **Alt+Shift+Space** steps to the next mode.

Before you click, what a click would pick lights up under the pointer. In
**Items** mode the whole item lights up.

## Selecting with the mouse

| Do this | To |
|---|---|
| Click | Select what's under the pointer, and nothing else |
| **Shift**-click | Add to the selection |
| **Control**-click | Take it out of the selection |
| Drag across the mesh | Select everything the pointer passes over |
| Right-drag | Draw a lasso; what is inside and in front is selected |
| Middle-drag | Draw a lasso that also selects what is hidden behind |
| Double-click an edge | Select its whole loop (**Shift**: add the loop) |
| Double-click a polygon | Select the whole piece it belongs to |
| **Shift+Ctrl**-click a second polygon (**Shift+Cmd** on a Mac) | Select the rectangle of polygons between the two |
| Click empty space | Clear the selection |

**Esc** with no tool active also clears the selection. Each change to the
selection is one undo step.

## Tips

- With nothing selected, most tools work on the whole mesh.
- **Alt+1**, **Alt+2** and **Alt+3** turn the current selection into points,
  edges or polygons and switch to that mode.

## Related

- [Select tab](select-tab.md)
- [Loops and rings](loops.md)
- [Interface](../interface/window.md): the **Mode bar**
