# Shader layer menu: GUI checks

For Jason or Codex. QA build named in the report, a throwaway scene with a
cube and a sphere, the Shader Tree panel open. Right-click a layer's name
(not the small "+" on a texture row) for its menu. "Render" = Render >
Render Viewport.

## Stage 1

1. Right-click Base Material → Rename…, Editor Color ▸, Properties,
   Duplicate, Create Instance, Copy, Delete, Presets ▸, in that order; no
   Select Polygons, Paste or image entries.
2. Duplicate → "Base Material (2)" appears directly above, selected. Change
   its colour: the original doesn't change. One Cmd+Z removes it.
3. Editor Color ▸ Teal → the row is tinted teal. Save, reopen → still teal.
4. Copy a texture layer, select a layer inside a group, Paste → the copy
   lands directly above it; its locator row (+) names its own locator.
5. Copy a Constant, select another Constant, Paste Values → it takes the
   colour and opacity, keeps its name. On a Material → status "Paste Values
   needs a layer of the same type".
6. Assign Material "Red" to some polygons, right-click the "Red" group →
   Select Polygons → those polygons selected in Polygon mode.
7. Properties → the Properties panel comes to the front on that layer.
8. Image Map with a picture: Reload Image, Replace Image… (keeps its
   settings) and Open Image Folder (Finder shows the file).

## Stage 2

9. Right-click a Constant → Change Type ▸ Textures ▸ Noise → same place,
   same opacity, blend, effect and locator; Noise settings at their
   defaults; named "Noise" (or "<name> (2)" if renamed). One Cmd+Z.

## Stage 3

10. Create Instance on a Constant → an italic "… (2)" directly above.
11. Change the source's colour → the instance follows. Change only G on the
    instance → its Color label goes italic; later source colour changes
    reach R and B only.
12. Right-click the italic Color label → Revert to Source → it follows
    again. "Revert All to Source" clears every override.
13. Select Instances on the source / Select Source of Instance on the
    instance select each other.
14. Delete the source → "Also deletes 1 instance." OK deletes both; one
    Cmd+Z brings both back. Save, reopen → instance still italic and
    linked.

## Stage 4

15. Drag a Material Group onto the Library heading → it leaves the tree
    and shows under Library (open its triangle); renders no longer show it.
16. Drag it from the Library into the tree (above Base Material) → a
    reference row "R"; renders show the group's look. Edit the Library
    group's material → the render changes.
17. Right-click the reference → References ▸ De-Reference → a real group in
    its place; editing the Library no longer changes it.
18. Right-click the Library item → Lock Preset → its Properties grey out
    and edits are refused with a status message; Unlock Preset undoes it.
19. Delete a Library item that has a reference → Yes keeps a copy, No
    deletes the reference too, Cancel changes nothing.
