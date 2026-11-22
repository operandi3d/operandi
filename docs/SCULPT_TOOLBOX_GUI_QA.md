# Sculpt toolbox brushes — GUI check (spec sculpt-toolbox-2026-11-11)

For Jason or Codex (Claude's screen control can't click in Operandi). Use
the QA app named in the thread, with its own data folder. Start from a new
scene with a Sphere and a Plane, each subdivided a few times, Sculpt tab
open.

1. **Toolbox.** Rows read Push · Inflate / Fold · Clay / Smooth · Smudge /
   Carve · Scar / Flatten · Tangent Pinch / Emboss · Attenuate / Spin ·
   Move. The pinch button has a small corner triangle; right-click (or
   press and hold) lists Tangent Pinch and Center Pinch, and the button
   then shows the one picked. Attenuate is greyed; its tooltip says it
   needs a sculpt layer.
2. **Forms.** Each new brush opens Tool Properties with its spec rows
   (Clay and Scar have no Sculpt Mode row; the pinches have no Smooth Mode
   row). Sculpt Mode's image choices and Curve Constraint's background
   choices are greyed.
3. **Fold** on the sphere: a dent ringed by a raised rim. Ctrl: a bump
   ringed by a trough.
4. **Clay** on the plane, Level 10 %: builds up and stops at a flat top;
   brushing longer in one stroke doesn't go higher. Level 0 %: nothing.
5. **Carve**: back and forth in one stroke reaches the same depth as one
   pass; a second stroke goes deeper. Ctrl raises a ridge.
6. **Flatten** with Normal Mode Mouse Down over bumps: a plateau at the
   press point's height and tilt. Inverted Flatten on: dips fill instead.
7. **Center Pinch** gathers grid lines toward the centre without leaving
   the surface; **Tangent Pinch** draws them toward the stroke line.
8. **Spin** swirls grid lines; Ctrl the other way; Normal Mode Screen can
   fold the surface.
9. **Emboss**: press and drag right raises the patch under the press, left
   lowers it; the patch stays put; one undo step.
10. **Smudge** drags the surface along with the brush; **Scar** cuts a
    pinched gash; Ctrl gives a welt.
11. **Every brush**: Shift smooths (undo reads "Smooth Stroke"); Lock
    Borders / Corners hold the plane's edges; Symmetry X mirrors the
    stroke; pen pressure scales strength.
12. **Move** with Move Mode Slide on the sphere: the grabbed patch slides
    over the surface instead of lifting off.
