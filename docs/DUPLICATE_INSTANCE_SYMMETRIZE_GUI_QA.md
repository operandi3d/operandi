# Instance duplicates and Symmetrize: GUI checks

For Jason or Codex (Claude can't click Operandi). Use the QA build of the
merged Duplicate panel branch, on a throwaway scene. Each line: do this →
expect that. Spec: BEHAVIOR-DUPLICATE-PANEL (DX-30..45). The buttons and
flyouts are part A's checklist; this one checks what they start.

## Instance versions (DX-30..37)

1. Cube, Item mode. Duplicate tab ▸ Clone flyout ▸ Instance Clone (or
   Alt+click Clone). Tool Properties shows an **Item** section with
   Instance ticked, Parent Off, Hierarchy off.
2. Drag in the view → the copies appear; Drop. The Item List gains
   "Cube (2)", "Cube (3)"… next to the source.
3. Polygon mode on the source, move a few points → every instance changes
   the same way. Move one instance as an item → only that one moves.
4. Plain Clone afterwards → Instance is unticked (the plain buttons always
   start with it off).
5. Parent Hierarchy → each copy sits under the one before in the Item
   List; Root → all under the source; Primary → a new Group Locator under
   the source holds them. The copies don't jump when Parent changes.
6. A source with a child item, Hierarchy ticked → each copy gets a copy of
   the child under it.
7. Instance Mirror on a cube away from the origin → the copy appears
   mirrored and its faces look right (not inside-out) in the view and in a
   Cycles render (DXQ-2).
8. Instance Array, Instance Radial Array, Instance Scatter, Curve / Bezier
   / B-Spline Instance, Pen Instance: same checks as 1–2 (the path ones
   take clicks for the path first).
9. Undo after Drop → all the new instances go in one step. Save, reopen →
   they are still instances (edit the source, they follow).
10. Item mode, select two meshes, Shift+D (Item ▸ Instance) → one instance
    of each, in place, now selected; one undo step.

## Symmetrize (DX-40..45)

1. Box, Polygon mode, move it so it straddles X = 0 unevenly. Duplicate
   tab ▸ Symmetrize → the result shows at once: the + side kept, mirrored
   over, welded at X = 0 (no seam, nothing inside-out).
2. Tool Properties: Axis X, Direction Positive to Negative, Merge on,
   Offset 0, Topology off. Change each → the result follows live.
3. Direction Negative to Positive → the − side is kept instead.
4. Merge off → the seam stays open (Select ▸ Boundary finds it).
5. Select a single polygon first → it is ignored; the whole mesh is
   symmetrized (DXQ-3, Jason's OK).
6. Drop → one undo step "Symmetrize"; Cancel (Esc) instead → nothing
   changed.
