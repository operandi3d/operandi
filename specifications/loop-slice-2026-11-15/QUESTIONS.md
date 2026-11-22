# Loop Slice + drag layer: open questions (2026-11-15)

## A. Scriptable (bridge; needs Jason's "go", no hands needed)

- **LS-Q07** Is the first slice shown as Current 0 or 1?
- **LS-Q08** Two "only when Slice Selected is on / off" rules exist: which
  options do they grey (probably Slice N-gon and Keep Quads)?
- **LS-Q10** Inset and profiles, by numbers: with no profile, what does
  Inset 0.1 do to the cut? With a simple profile, where do the loops land
  and how deep (Keep Aspect on/off, both Reverse switches)?
- **LS-Q11** Geometry checks on a small grid: Keep Quads, Split + Caps + Gap,
  Mode Symmetry with Count 2 at 30%, Mode Uniform with Count 3, the Uniform
  button after Free slices.

## B. At the mouse (Jason, about 5 minutes; Loop Slice on a 3x3 grid)

1. **LS-Q01** With the tool live, click a different edge on the mesh. Does
   the cut jump to that ring, add a second ring, or nothing?
2. **LS-Q02** Left-drag sideways off the strip, slowly, about a hand's
   width. Does the slice follow the pointer about 1:1 with the strip,
   faster, or slower? Does dragging right move it toward the strip's
   right end?
3. **LS-Q03** Can you drag the whole strip somewhere else (not just its
   end dots)?
4. **LS-Q04** Right-drag sideways: roughly how far per extra slice
   (a finger-width? more?).
5. **LS-Q05** Try the left-drag again holding Ctrl, then Shift. Anything
   different (finer steps, snapping to 25/50/75%)?
6. **LS-Q06** Set Edit to Add and click on the mesh (not the strip). New
   slice or nothing? Same for Remove.
7. **LS-Q09** Mode Uniform, Count 3: drag a triangle. Do all three spread
   or squeeze, or is it locked? Mode Symmetry: does the partner move the
   opposite way?

## C. Drag layer, any tool (same session)

- **DL-Q01** Start a drag diagonally: does it lock to whichever way you
  moved first?
- **DL-Q02** How fast do whole numbers step (Polygon Bevel right-drag
  Segments is a good test)?
- **DL-Q03** Right = more and up = more, always?
- **DL-Q04** On the Mac trackpad: what gives you a middle-drag and a
  right-drag? (Cmd? Ctrl? Option? Two fingers?)
