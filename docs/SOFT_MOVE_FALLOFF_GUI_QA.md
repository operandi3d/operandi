# Soft Move's own falloff: GUI checks (TP-19)

Mug tutorial step. Branch `soft-move-falloff`.

1. Open a mug (or any mesh) in the Front view, handle to the left. Polygon
   mode, nothing selected, no falloff chosen.
2. Deform tab ▸ Soft Move. Status bar: "Click in a view to start Soft
   Move…". Nothing is drawn yet (CS-01).
3. Click above the left edge of the handle, level with its top. The Move
   handle and the falloff's rings appear centred on that click. Tool
   Properties shows Move, then a "Radial Falloff" section (FO-10).
4. The falloff has a red X box, green Y box and blue Z box on its rings,
   drawn over the Move arrows. Hovering a box gives its tooltip (UIC-05).
5. Drag the red X box toward the centre: the rings narrow on X only, until
   they cover just the handle. The Move handle doesn't move. Size X in Tool
   Properties follows.
6. Press the Move handle's centre or arrow: it is the Move that drags (not
   the falloff centre). The handle bends smoothly, full at the centre,
   fading to nothing at the rings; the mug body stays.
7. Undo: the move undoes. Drop (Space): the falloff goes away, Falloff menu
   shows (none) again (TP-12).
8. Pick Falloff ▸ Radial yourself with plain Move: its centre handle still
   drags the falloff when no Move handle is under the pointer.
