# Items list header: GUI checks

For Jason or Codex (Claude can't click Operandi). Use the QA build named in
the report, on a throwaway scene. Each line: do this → expect that.

1. Items tab tool row reads: Add Item, ⏷ at the left; Select, Filter,
   Sel, Notes (greyed), F at the right (the funnel replaced Text, UIC-132).
   There is no Item button.
2. Click Add Item (the label) → menu: find field, Recent ▸, Backdrop Item,
   Camera, Mesh, separator, Lights ▸, Locators ▸ and nothing else: no
   greyed entries, no Show unsupported (IL-Q4). The ⏷ opens the same menu.
3. Recent ▸ on a fresh data folder → one greyed "(none)".
4. Select the first mesh, Add Item > Mesh → "Mesh (2)" appears directly
   under the first mesh, alone selected. Cmd+Z removes it in one step.
5. Lights ▸ → Area, Cylinder, Directional, Dome, Mesh Light, Photometric,
   Point, Portal, Spot, all enabled. Locators ▸ → Group Locator only.
6. Type "spot" in the find field → only Spot Light; Enter adds it. Recent
   now lists Spot Light first, then Mesh. Quit and relaunch → Recent is
   the same.
7. Type "vortex" in the find field → "(no match)": types Operandi can't
   make aren't listed.
8. Select popover: Pattern `cam*`, Select → the camera(s) selected. Pattern
   `m?sh*`, Hide → meshes hidden; Cmd+Z brings them back in one step.
   Group, By Color and Color are greyed.
9. Parent a mesh under another, select the parent, tick Limit to current
   selection child items, Pattern `*`, Select → only the child selected.
10. Filter popover: untick Cameras → camera rows go, selection unchanged.
    Flatten → every row at the left, a child shows "· Name". Sort by Name →
    alphabetical. Relaunch → the filter is still on.
11. Hide the parent's type (or use Sel with only the child selected) → the
    parent stays as a grey row above the child.
12. F → a Search row with Flat appears; typing narrows the list; Flat
    drops the grey parents; F off → row gone, full list back.
13. Right-click an item row → Delete Item and Rename… are still there.
