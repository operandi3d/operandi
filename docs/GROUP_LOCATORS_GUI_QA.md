# Group Locators: GUI checks

For Jason or Codex (Claude can't click Operandi). Use the QA build named in
the report, on a throwaway scene with two or three meshes. Each line: do
this → expect that. Design is provisional (GL-Q1..GL-Q5 in
SPECIFICATION_QUESTIONS.md).

1. Items tab → Add Item ⏷ → Locators → Group Locator (Locator and Texture
   Locator stay greyed) → a "Group Locator" row after the selected item,
   alone selected, with a little blue folder icon (the Texture Group's
   folder is grey, not blue). A second one is
   "Group Locator (2)". Cmd+Z removes it in one step.
2. In the viewport the locator shows as a cube drawn only by its corner
   brackets plus a small centre cross, the same size on screen whatever
   the zoom; orange (selection colour) when selected. Preferences >
   Display > Item Drawing size makes it bigger.
3. Select two meshes (Ctrl+click rows), Cmd+click the viewport in Items
   mode → Item Operations → Parent to Group Locator → a new locator takes
   the first mesh's row place, both meshes are its children, nothing
   moved, only the locator is selected. One Cmd+Z undoes it all.
4. Right-click a mesh row → Parent to Group Locator is there and does the
   same for the selection (the clicked row joins it if it wasn't
   selected).
5. With the locator selected pick Move, Rotate or Scale in the toolbox in
   Polygon mode → the selection mode switches to Items and dragging moves,
   turns or scales the locator; its children follow and its bracket cube
   turns and scales with it.
6. Locator selected, try Bevel or another component tool → status says
   "Cameras, lights and locators have no geometry; …".
7. Properties: tabs read Group Locator, Display, Assembly, Channels, Tags;
   the Group Locator tab has Name and Item Transform rows and one grey
   note line.
8. Viewport Display menu (top-left of the view) → Show Locators off → the
   locator shapes disappear in that view only; the other view still shows
   them. Display tab > Visible: Yes draws it anyway.
9. Filter popover (Items header) → untick Locators → locator rows hide
   (children that still match stay under a greyed parent row). Select
   popover's Locators tick narrows pattern matches the same way.
10. Save, close, reopen → locators, hierarchy and placement come back.
11. Export OBJ → children are where they appear in the view. Export LXO →
    a note says the Group Locator(s) were left out; children keep their
    places.
12. Delete (Backspace, or right-click > Delete Item) a locator that has
    children → a "Delete Item" dialog: "The item Group Locator has child
    items. Do you want to delete the child items too?" with Yes, No and
    Cancel stacked on the right, keys shown at each button's right (Ret,
    Esc, Shift+Esc). Shift+Esc or Cancel → nothing changes. No (or Esc) →
    the locator goes, its children sit at the top level where they were,
    and a child's own children stay under it. Yes (or Return) → the
    children go too. Cmd+Z undoes either in one step. Works the same for a
    mesh with children.
13. Select two parents (Ctrl+click) → Delete → the dialog says "Some of
    the selected items have child items. …"; the answer applies to both.
14. A childless item deletes at once, no dialog.
