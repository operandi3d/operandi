# Addendum: general tools in a UV view (UVT-130..139)

Received 2026-11-16 (from research, via the project thread).

**UVT-130 One pipe, two spaces.** As in BRIEF-TRANSFORM-PIPE-IN-UV.md.

**UVT-131 Element Move in the UV view.**
- Hovering highlights the UV point, edge or polygon under the pointer
  (Automatic mode picks whichever is under it; Manual uses the selection mode).
- Click-drag moves it. A click with no drag shows the handles on it.
  Right-drag sets the Element Falloff Range.
- Range is in UV units (guess, UVT-Q11). At Range 0 only the clicked element
  moves.
- Use Connectivity means the clicked UV island only (guess, UVT-Q12). Rigid
  Connections moves the whole island.
- At a seam, only the clicked UV position moves.

**UVT-132 Falloffs in UV.**
- Linear and Radial are placed and measured in U/V.
- Screen falloff is unchanged.
- Element falloff works as UVT-131.
- Selection (soft selection) falloff counts steps over UV edges (guess).
- Vertex Map falloff reads per-3D-point weights as in 3D.

**UVT-133 Action centres in UV.**
- Selection: the UV selection's centre.
- Element: the clicked UV element.
- Origin and Pivot: UV (0,0).
- Screen and Work Plane fall back to the selection centre (guess).

**UVT-134** Flat handles, coloured teal for U and yellow for V.

**UVT-135** X/Y fields edit U/V; Z fields are greyed (as for Jitter's Range Z).

**UVT-136** Symmetry in a UV view is UV symmetry only (off until it's built).

**UVT-137 Slide on UVs.** Slide gets a UV Map field. (none) slides in 3D; a map
name slides that map's UVs along their UV edges. In a UV view it starts on the
current map (guess). Stage 3 in the plan; do it as a separate last batch only
if it's cheap once stage 1 is green, otherwise list it as the next batch.

**UVT-138** Slip UVs (Element Move, Slide) is a 3D feature and is greyed in a
UV view.

**UVT-139** Sculpt tools on UVs: later.
