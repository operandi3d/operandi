<!-- help-ids: Items List ; template: panel ; since: 0.1.0 -->
# Items list

The **Items list** shows every item in the scene (meshes, cameras, lights,
locators, groups) as a tree, with children under their parents. It is the
**Items** tab of the panel at the top right.

![The Items list with one mesh, Cube, selected](../images/panel-items.png)

## Parts

| Part | What it does |
|---|---|
| **Add Item ▾** | Adds a new item; the arrow lists every kind (mesh, camera, light, locator, group and more). |
| **Select**, **Filter**, **Sel** | Select items by type, show only some kinds of item, and show only the selected items. |
| **Notes**, **F** | Show the items' notes column; **F** is a filter field to find items by name. |
| Eye column | Click to hide or show an item; drag down the column to change several. |
| Name column | The items, as a tree. The scene itself is the top row. |

## Common tasks

- **Select an item**: click its name. **Shift**-click selects a range;
  **Control**-click (or **Cmd**-click on a Mac) adds or removes one item.
- **Rename**: double-click the name and type.
- **Parent an item**: drag it onto another item; drag it to the top level
  to unparent it.
- **Parent or sort from the 3D view**: in **Items** mode, press on an
  item in the **3D view** and drag it onto a row of the list: onto a row
  parents it there, between two rows puts it there.
- **Hide an item**: click its eye.
- **Right-click** a row for that item's menu.

Pressing **5** switches to Items mode, where clicks in the 3D view select
whole items.

## Related

- [Interface](../interface/window.md): part 10 is the Items list
- [Properties panel](properties.md)
- [Merge and Unmerge](../modeling/merge-unmerge.md)
