# Pie menus (stage 1): GUI checks

For Jason or Codex (Claude can't click Operandi). Use the QA build named in
the report (`Operandi pie-menus <commit>.app`), on a throwaway scene. Each
line: do this → expect that. "Ctrl" is the Control key, not Cmd.

1. Perspective view, pointer mid-viewport. Hold Ctrl+Space → a pie opens
   at once, centred on the pointer: Top at the top, then clockwise
   Perspective, Right, Front, Bottom, Back, Left, Camera (greyed with no
   camera). Perspective shows a filled dot.
2. Still holding, move up → Top lights orange, a small arrow points at it.
   Release → Top view; pie gone. Cmd+Z does not undo the view change.
3. Tap Ctrl+Space (release without moving) → the pie stays. Click Front →
   Front view, pie closes. Tap again, press Esc → closes, nothing changes.
   Tap again, click far away → closes, nothing changes.
4. Hold Ctrl+Space, move out to Right, come back to the centre, release →
   closes, no change.
5. Pointer near a window corner, Ctrl+Space → the whole pie is on screen
   (pie moves, pointer doesn't).
6. Alt+Q → Edges, Polygons, Items, Move, Rotate, Scale, Transform, Vertices
   (current mode dotted). Flick to Move → Move tool on, as W.
7. Ctrl+2 → shading styles; flick Gooch → the view draws Gooch, the
   Display button says Gooch. Ctrl+1 → Vertices shows an empty box; flick
   it → points show, box filled next time.
8. Edge mode, bind Pie: Modeling in Input Map, open it → Bevel greyed;
   releasing over it closes with nothing.
9. Input Map: search "Pie" → seven pies. Assign W to Pie: View Direction →
   the clash banner names Move.
10. Preferences > Defaults > UI, Popup Transparency 50 % → the pie's
    labels are see-through like menus.
11. Open a dropdown (e.g. the view type menu), press Ctrl+Space → the
    dropdown closes and the pie opens.

## Hold and release, tight layout (2026-11-15)

12. Hold Ctrl+Space, move onto Right, let go of both keys → Right view,
    no click. Repeat letting go of Ctrl first, then Space → same.
13. Quick flick: tap-hold Ctrl+Space, flick up and let go fast → Top view;
    the pie never sticks open.
14. Same hold-point-release with Alt+Q (Move), Ctrl+2 (Gooch), Ctrl+1 →
    each runs the slot let go over.
15. Look: the view pie is a tight cluster of rounded buttons around a
    small dot (Top/Bottom just above/below, Left/Right level, diagonals
    between), about 400×220 px. Action Centers' "Selection Auto Axis"
    fits its button.
16. Pointer on the lower-right corner of Perspective (inside the button)
    → Perspective lights, not Right.
