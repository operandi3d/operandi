# Pen tablet — GUI QA checklist

Branch `pen-tablet`. Use the QA build with its own data folder; a Wacom (or
any tablet macOS drives) plugged in. Start the build from Terminal with the
readout on:

```bash
OPERANDI_PEN_DEBUG=1 open -n "/Volumes/Development/Operandi Builds/Operandi pen-tablet <sha>.app"
```

(`open` passes the variable only on recent macOS; if the readout doesn't
show, run the binary inside the bundle: `OPERANDI_PEN_DEBUG=1 ".../Contents/MacOS/operandi-app"`.)

## Readout (stage 1)
- [ ] Bottom-left of the viewport: "Pen: …" line. Before the pen comes
  near: "Mouse … no pen seen yet".
- [ ] Hover the pen: tool reads "Pen tip", "in proximity"; tilting the pen
  moves the tilt numbers (−1..1); a pen with barrel rotation moves rotation.
- [ ] Press harder and lighter on the tablet: pressure moves 0..1 smoothly.
- [ ] Flip to the eraser end: tool reads "Eraser". Lift away: "out of
  proximity", tool back to "Mouse".
- [ ] Preferences > Input > Pen Tablet > Pressure Curve: Soft makes the
  same touch read higher, Firm lower (raw stays the same).
- [ ] Without the variable the readout is not drawn.

## Sculpt (stage 2)
- [ ] Push on a subdivided cube: a light stroke raises less than a hard
  one; a stroke getting harder along its path ramps up smoothly (no
  steps between frames).
- [ ] Each stroke is one undo step ("Push Stroke"); Undo/Redo restore it.
- [ ] Eraser end on Push digs in (like Ctrl). Preferences > Eraser End =
  Smooth: the eraser smooths (undo label "Smooth Stroke").
- [ ] Smooth Mode dropdown in Tool Properties: Low Pressure is greyed with
  "Needs a pen tablet…" in a session where the pen hasn't come near the
  tablet; after the pen hovers once it un-greys. With it chosen, light
  strokes smooth and hard strokes sculpt.
- [ ] Pressure Sets Brush Size off (default): the ring stays one size while
  drawing. On: the ring and the dabs shrink with light pressure.
- [ ] Mouse sculpting unchanged: Push, Inflate, Smooth, Move with a mouse
  feel as before (full strength, ring fixed size), Ctrl/Shift as before.
- [ ] Trackpad with Force Touch: pressing harder doesn't change strength.

## Pen navigation (no code changes; check nothing regressed)
- [ ] Alt + pen drag orbits; the preset's pan and zoom chords work with
  the pen's side buttons / modifiers.
- [ ] A side button set to middle click: middle-drag pans (and with a brush
  active sets strength, SC-23).
- [ ] Taps with the pen select (and right-taps open menus) without turning
  into tiny drags; note any tool where a tap moves things.
- [ ] Hovering the pen pre-highlights elements under it like the mouse.
- [ ] The pen in floating windows / panels: clicks and field scrubbing work.
