# Material-value render outputs — GUI check

Build: the QA app named in the thread (Cycles build). Use a fresh scene
with a Cube.

1. Shading panel, Add Layer > Special > Render Output: today's 18 entries,
   a separator, then a **Material Values** sub-menu with 13 entries
   (Diffuse Amount ... dPdu, dPdv). Hover one: a one-line tip says what
   it writes.
2. Pick **Diffuse Amount**: a "Diffuse Amount Output" layer appears. Its
   Effect drop-down lists all 31 effects with Diffuse Amount ticked.
3. Render (no other change): the Render window's output menu has
   Diffuse Amount; the cube is a flat grey at the default material's
   Diffuse Amount (not black), the background black.
4. Base Material: Diffuse Amount 37 %, Roughness 42 %. Add Roughness and
   Diffuse Coefficient outputs, render: Diffuse Amount is a darker flat
   grey, Roughness a slightly lighter one, Diffuse Coefficient the diffuse
   colour, dimmed.
5. Add a Constant texture layer over the material set to Roughness,
   value 90 %: the Roughness output turns near white; the others don't
   change.
6. Add Transparent Color and Diffuse Energy Conservation outputs. Set
   Transparent Amount 50 %: Energy Conservation is a little under half
   the Diffuse Coefficient (specular and coat take their share
   too); Transparent Color shows the transparent colour.
7. Give the cube UVs (any unwrap) and add dPdu and dPdv outputs: each face
   is one flat colour (directions, so some faces are black where the
   values are negative); neighbouring faces differ.
8. Save, reopen: the outputs and their settings are kept. Cmd+Z after
   adding an output removes it in one step.
9. In the render folder: "Render N (Diffuse Amount Output).png" and the
   others are written next to "Render N.png".
