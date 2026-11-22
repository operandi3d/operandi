# Item transform Reset + Freeze — GUI check

Branch `freeze-transforms`. Spec: specifications/freeze-transforms-2026-11-14.
Use the QA app "Operandi freeze-transforms <sha>" in ~/Developer/Operandi-Builds,
on a new scene. Item mode throughout. After every Freeze, the thing to look
for is that **nothing moves on screen**; after every Reset, that the item
**does** move.

## 1. Where the controls are
1. Add a Box. Open Item Properties. Under the Position / Rotation / Scale
   rows of Item Transform there are two rows, **Reset** and **Freeze**, each
   a full-width dropdown reading "Position, Rotation, Scale, All".
2. Open each: Position, Rotation, Scale, All. No Shear.
3. Hover Freeze: the tooltip reads "Bake the transform into the geometry".
4. Pick an entry: it runs at once and the dropdown still reads "Position,
   Rotation, Scale, All".
5. Select the camera: only **Reset** shows (no Freeze). The camera's
   Camera View tab no longer has its own Reset.
6. Menu bar: Item > Transform lists Reset Position … Reset All, a line,
   Freeze Position … Freeze All.

## 2. Freeze, one item
1. Box: set Position 1, 2, 0; Rotation 0, 45, 0; Scale 200 %, 100 %, 100 %.
2. Freeze > Rotation. The box doesn't move. Rotation reads 0, Position and
   Scale unchanged.
3. Edit > Undo (title "Freeze Rotation"): the channels come back, the box
   still hasn't moved. Redo.
4. Freeze > All. Nothing moves; Position 0, Rotation 0, Scale 100 %. The
   item's centre marker is now at the world origin.
5. Freeze > All again: nothing happens and no new undo step appears.

## 3. Parent and child
1. New Box A at Position 2, 0, 0, Rotation 0, 0, 30, Scale 150 % uniform.
2. New Box B, drag it under A in the Items list, Position 0, 1.5, 0.
3. Select A only. Freeze > Position, then Rotation, then Scale. Neither box
   moves at any step; B's channels change to keep it in place.
4. Undo three times: back to the start, both boxes where they were.
5. Select both A and B (B first, then A). Freeze > All. Nothing moves; both
   read identity. One Undo restores both.

## 4. Mirrored cube
1. New Box, Scale -100 %, 100 %, 100 %. Shading looks normal (faces out).
2. Freeze > Scale. Scale reads 100 %; the box still shades the same, not
   inside out (check in Advanced / Cycles Preview too).

## 5. Offset pivot
1. New Box, then in Polygon mode move all its polygons 3 m along X, so the
   mesh sits away from its centre. Back to Item mode.
2. Rotation 20, 50, 10. Freeze > Rotation: nothing moves.
3. Undo, then Freeze > All: nothing moves.

## 6. Locked item
1. Two boxes, both scaled 200 %. Lock one (Assembly > Lock).
2. Select both, Freeze > Scale. The unlocked one now reads 100 % and didn't
   move; the locked one is untouched. The status bar says "1 locked item
   skipped".
3. Select only the locked box: Reset and Freeze are greyed with the "locked"
   reason.

## 7. Reset
1. Box with a child, Position 1, 1, 1, Rotation 30, 0, 0.
2. Reset > All: the box jumps to the origin and the child moves with it;
   the box's shape (its points) is unchanged.
3. Undo title "Reset Transform".

## 8. Mesh Ops
1. Box, add a Mirror mesh op, Position 2, 0, 0.
2. Freeze > Position: nothing moves and the Mirror op is still in the list
   and live.

## 9. Typed / script
1. Command line: `transform.freeze scale`, `transform.reset rotation`,
   `transform.freeze` (all). Each acts on the selected items as above.
2. `transform.reset shear` refuses with a message.
