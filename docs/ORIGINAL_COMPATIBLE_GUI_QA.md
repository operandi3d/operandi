# Original-compatible switch: GUI check

For Jason or Codex. QA build from branch `original-compat`.

1. Preferences > Defaults > UI: "Original-Compatible Behavior" is off. Under
   it: "When on, these change:" with A1, A4, A5, A6 and A7. Hover the row:
   tooltip. `pref.value ui.original_compatible ?` reads false.
2. Turn it on (or `pref.value ui.original_compatible true`).
3. A1: select a mesh, Move tool (W). At the default view the gizmo is the
   same size as with the switch off. Zoom in (wheel): the gizmo grows on
   screen with the mesh; zoom out: it shrinks, down to too small to see.
   Same for Rotate's rings, Scale's squares, the plane circles and a tool
   handle (e.g. Bevel). Top view: same with orthographic zoom. Handle
   Scale (Preferences > Display > Tool Handles) 2 doubles it at any zoom.
   Switch off: constant on-screen size again.
4. A2 (no change, already the default): double-click a colour field: the
   picker opens; no text box appears.
5. A4: add a Backdrop with no image. In its view it shows a 1 m square of
   fine light dots, no orange outline, also after changing Pixel Size.
   Select it: the selection outline round the dotted square. Give it a
   picture: the picture as usual.
6. A5: Backdrop with a picture, Show In Perspective on, a mesh in front of
   it. Perspective view: the picture covers the mesh; the floor grid in
   front of the picture still shows over it. Top/Front/Side: the mesh over
   the picture, as before. Camera view: as before. Show In Perspective
   off: hidden.
7. A6 (fresh app start): give some polygons part "Wheel", select them, Set
   Part...: the Name field shows the Default Part preference, not "Wheel".
   OK with "Hub". Select other polygons, Set Part...: "Hub".
8. A7 (Mac): Item List with three items. Cmd+click toggles a row. Ctrl+click
   on a row opens its right-click menu and leaves the selection alone. Same
   in the Shader Tree and Groups lists.
9. Turn the switch off: everything back to today's behaviour without a
   restart (Ctrl+click toggles again; Set Part prefers the shared part).
