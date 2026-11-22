# Build brief: item Reset + Freeze (stage 1)

Approved by Jason 2026-11-14: build Reset and Freeze now. Zero, Add, Shear and the transform layer stack come later (stage 2).

Full behaviour: BEHAVIOR-ITEM-TRANSFORM-OPS.md, sections 3, 4, 7 and 8. Follow the [REC] lines where they differ from [OBS]. The open-question defaults in section 10 stand unless Jason says otherwise.

## Scope
1. UI. Add an Item Properties Transform section to every item type that has transforms, under the Position, Rotation, Order and Scale rows:
   - a Reset dropdown on all item types, with Position, Rotation, Scale and All;
   - a Freeze dropdown on geometry items only (mesh-type), with the same entries.
   - Each pick runs the action straight away. Hide the Shear entries until shear exists. Follow the bible and cite UIC rules.
2. Commands. Register `transform.reset <translation|rotation|scale|all>` and `transform.freeze <translation|rotation|scale|all>` (no argument means all). Both act on the item selection. Reuse the existing camera ResetTransform logic for Reset, and move the camera UI onto the shared command.
3. Reset. Primary channel goes to identity (scale to 100 %). Children are not compensated. Geometry is untouched.
4. Freeze. Bake the item's local transform for the chosen type into the vertices, then set the channel to identity. Nothing moves on screen.
   - Children: rewrite their channels so they keep their world placement.
   - Parent and child both selected: do the parent first.
   - Negative scale (an odd number of negative axes): reverse polygon winding.
   - Transform vertex normals and relative morph offsets the same way. UVs and weights are untouched.
   - Pivot offset: include it, so the mesh never jumps.
   - Instances of the item: compensate so they don't move.
   - Animated channel: skip it, with the status message "Position is animated, Freeze skipped".
   - Locked item: skip it, with one status note giving the count.
   - Items with Mesh Ops or deformers: bake into the base cage, and the stack stays live.
   - Rotation-only freeze under non-uniform scale: if that can't be done exactly, bake what is exact and say so.
   - Already at identity: no change and no undo step.
5. Undo. One step per pick, named like "Freeze Rotation". Undo restores everything exactly.
6. Menu. Add an Item > Transform submenu with the same entries.
7. Naming. The existing geometry Freeze (SubD, curves, Mesh Ops) keeps its name. The tooltip for the new Freeze reads "Bake the transform into the geometry".

## Tests
- Freeze each type with a parent and child: the child's world matrix is unchanged and the parent's vertices land in the right place.
- Mirrored cube: outward normals are kept after Freeze Scale.
- Offset pivot plus rotation: world vertex positions are unchanged after Freeze Rotation and after Freeze All.
- Instance: world placement unchanged.
- Keyed channel: skipped, keys untouched.
- Locked item: skipped.
- Multi-select: each item frozen on its own, and one undo restores all of them.
