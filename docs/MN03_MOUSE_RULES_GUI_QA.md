# Mouse rule gaps (MN-03): GUI checklist (2026-11-19)

For Jason or Codex, on the QA build.

1. Cube: draw one. Middle-click elsewhere on the grid: the first cube stays
   and an identical one (size, segments) appears at the pointer. Middle-drag
   instead: the copy appears and the drag resizes its nearest side. Same
   for Sphere and Cylinder. Before anything is drawn, a middle-click says
   to draw one first.
2. Move a polygon with a Linear falloff on: right-drag sets the falloff
   from the press to the pointer. Radial/Cylinder: centred on the press,
   sized by the drag. Screen: the circle. With no falloff, right-drag does
   what it did before.
3. Bevel with the Screen falloff on: a drag bevels; a second plain drag
   starts a fresh bevel on top; a Shift drag keeps adjusting the last one.
   Without the Screen falloff Bevel behaves as before.
4. Airbrush falloff with Move: a drag paints weight; Shift-drag lowers it;
   Cmd-drag (Ctrl on Windows/Linux) erases it back to none; right-drag
   sizes the brush; middle-drag is Move's own (repeat last drag).
