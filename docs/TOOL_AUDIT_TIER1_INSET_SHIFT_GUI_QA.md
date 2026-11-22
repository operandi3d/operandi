# Tool audit Tier 1: Inset and Shift GUI checks

For Jason or Codex (Claude can't click Operandi). Use the QA build named in
the report, on a throwaway scene. Each line: do this → expect that.

Neither tool has a toolbox button. Start them from the modeling popup menu
(Polygon mode), the menu bar's Geometry > Polygon > Shift / Inset, or by typing `inset` / `shift` (or `tool.set poly.inset on`)
on the command line.

1. Polygon tab: no Inset or Shift button; the tab looks as on main.
2. Cube, select the top polygon, start Inset from the modeling popup. Tool Properties: one field, Inset
   0 m. Type 0.2 → a smaller square framed inside the top; it stays
   selected.
3. Drag the Inset handle (or drag anywhere left/right) → the inner square
   shrinks/grows; negative grows past the original.
4. Cmd+Z inside the tool takes back the last drag or typed value; after
   Drop, one Cmd+Z removes the whole inset.
5. Deselect everything, type `inset` on the command line, Inset 0.1 → every polygon of the cube gets one.
6. Select two neighbouring polygons, start Shift from the modeling popup,
   Shift 0.3 → each moves out along its
   own normal with its own side walls (a gap opens at the shared corner
   edge); the moved polygons stay selected. Drag the handle / drag anywhere
   up-down → the distance follows.
7. Open the modeling popup menu in Polygon mode → Shift and Inset
   are enabled and start the tools.
8. Menu bar Geometry > Polygon → Shift and Inset listed after Thicken;
   each starts its tool.
9. Escape during either tool → the mesh is as before.
