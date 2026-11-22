# Item editor colours: GUI checks

Branch `item-editor-colors`. Any scene with a few meshes, one parented.

1. IC-01: select two items, right-click one > Editor Color > Orange: both
   rows get an orange bar at the left of the name and a faint orange
   tint; the bar stays on the selected rows. Cmd+Z once clears both.
2. IC-02: Items list Filter (funnel) > By Color on, Orange: only orange
   items, with their parents greyed; By Color off restores the list.
3. IC-03: Select popover > By Color on, Orange > Select: exactly the
   orange items are selected.
4. IC-04: Save, close, reopen: the colours are back.
5. Shader Tree: right-click a layer > Editor Color > Blue: the row is
   marked; several selected layers change together, one undo step.
6. Groups tab: a member row shows its item's colour; its right-click menu
   has Editor Color.
7. Command line: `item.editorColor` reports the selected item's colour;
   `item.editorColor color:lightblue` sets it; `none` clears.
8. Preferences > Colors > Item Editor Colors: change Red; red rows follow.
9. Viewport and render look the same whatever the colours.
