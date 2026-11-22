# Item transform operations (behaviour spec, relevant parts)
Tags: [DOC] documented, [OBS] observed, [REC] our recommended behaviour (wins where they differ).

## 1. Where it lives
Item Properties, Transform section, directly under the Position / Rotation / Order / Scale rows. Dropdown buttons, stacked, full width of the value column. Reset: Position, Rotation, Scale, (Shear), All. Freeze: Position, Rotation, Scale, (Shear), All. (Zero and Add are stage 2, not now.)
- The button label never changes; it is an action menu, not a value picker. Picking an entry runs the action immediately and the menu closes.
- Freeze only works on items that own geometry. On a locator it is disabled.
- Also script commands (section 8).

## 2. Shared concepts
Identity values: Position 0,0,0; Rotation 0,0,0 deg; Scale 100 %; Shear 0.
"All" means every type in that button's menu, done as one action.
Rotation Order is never changed by any operation. [REC]

## 3. Reset
Puts the chosen channel(s) back to identity. The item visibly moves.
- All: position, rotation and scale go to identity.
- Geometry is not touched.
- Children are not compensated; they move with the parent, as if the user typed the values. [REC]
- Animated channel: behaves like typing the value (sets the current-time value, keys it only if auto-key is on). [REC]

## 4. Freeze
Bakes the chosen transform into the vertices, then sets the channel to identity. The item does not visibly move.
### 4.1 Per entry
- Position: vertices are offset by the position, measured in the item's own axes (so rotation and scale stay valid). Position becomes 0. Rotation and scale unchanged.
- Rotation: vertices rotated so the item looks the same. Rotation becomes 0. With non-uniform scale, the scale channel stays and the vertices absorb the difference.
- Scale: vertices scaled. Scale becomes 100 %.
- All: the complete local transform is baked. All channels go to identity, so the item's center is now at its parent's origin.
Builder note: freezing rotation alone while non-uniform scale stays on the channel can't always be exact (it can need shear). Then bake as much as is exact and say so in the status bar, or recommend Freeze All. [REC]
### 4.2 Rules
1. Local only: only the item's own transform is baked, never the parent's.
2. Children keep their world placement: each direct child's channels are rewritten so it does not move.
3. Parent and child both selected: parent first, then the child is frozen in its new compensated state. End result: nothing moves and both read identity.
4. Negative scale: polygon winding reversed during the bake so faces still point outwards.
5. Normals and other vector data (vertex normals, relative morph offsets) transformed the same way. UVs, weights, selection sets, materials untouched. [REC]
6. Pivot: bake the full transform including the pivot, so the mesh never moves. After Freeze All the pivot is reset to the center. After freezing one type, the pivot keeps its position on the mesh in world space. [REC]
7. Instances: compensate instances so they do not visibly move. [REC]
8. Animated channels: refuse that channel, status message ("Position is animated, Freeze skipped"). [REC]
9. Locked items: skip, one status-bar note. [REC]
10. Mesh Op / deformed items: bake into the base cage only, keep Mesh Ops and deformers live. [REC]
11. No-op: if already at identity, nothing changes and no undo step. [REC]
12. This is a different action from the existing geometry Freeze (SubD/curves/Mesh Ops to polygons).

## 7. Common rules
- Multiple selection: acts on every selected item independently; the whole action is one undo step.
- Locked items skipped, one status-bar note giving the count.
- Items that can't take the action (Freeze on a locator) are skipped silently when mixed with valid items. If nothing valid is selected, the entry is greyed out.
- Undo: one step per pick, named e.g. "Freeze Rotation". Undo restores geometry, channels, children and instances exactly, no stale display.
- Symmetry, falloff, action centre and snapping do not apply (item-level actions).

## 8. Script commands
transform.reset translation|rotation|scale|all ; transform.freeze same, no argument = all. Both act on the current item selection. (shear values reserved for later.)

## 10. Defaults
Q1 keep "Freeze" label in the Transform section; command names differ; tooltip "Bake the transform into the geometry". Q2 children compensated. Q3 instances compensated. Q4 animated channel refused with message. Q5 two stages (this is stage 1). Q6 hide Shear. Q7 bake into base cage, stack live. Q8 bake pivot properly, never jump. Q9 skip locked. Q10 Item > Transform submenu too.
