# Number field gaps (MN-06): GUI checklist (2026-11-19)

For Jason or Codex, on the QA build.

1. Rotate a polygon (E), drag the angle field's arrows: the value pauses
   at 0, 30, 45, 90 … (90 and 0 hold longer). Move's offset pauses at 1 m.
   Shift (faster) and Option (slower) still work.
2. Click the right half of Move's offset arrows: +10 cm; left half: −10 cm;
   with Shift: ±1 m. Whole-number fields still step by 1.
3. Item mode, a box selected. Item Transform's Position X row has a small
   link button before its arrows (chain icons since MN-06b). Each click
   goes off, same value, scale together, add together, off; its tooltip
   names the mode.
4. Copy: type 3 in Position X, Enter: X, Y and Z are all 3, one Undo.
5. With the button on Independent: Alt+Enter adds the same change to all
   three, Cmd+Enter scales all by the same ratio, Cmd+Alt+Enter copies.
   Plain Enter changes only that axis. The button doesn't change.
6. Relative: drag Position X's arrows: all three axes move together.
