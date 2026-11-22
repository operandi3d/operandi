# Render outputs group D: GUI checklist (2026-11-20)

For Jason or Codex (Claude can't click in the app). QA app: the
render-outputs-d build. Start from a scene with a floor (a flat box), a cube
standing on it, one point light above and to the side of the cube, and the
default environment. Decisions: RO-Q18..RO-Q31 in SPECIFICATION_QUESTIONS.md.

1. Shading panel > Add Layer > Special > Render Output lists Illumination
   (Unshadowed), Diffuse Shading (Unshadowed), Light Path Expressions and
   Shaded AA Samples; each adds "<name> Output" at the top of the tree.
2. Add Illumination (Direct) and Illumination (Unshadowed). Render (F9):
   the render window says "Unshadowed pass" for a moment after the main
   render. Direct has a dark shadow patch beside the cube; Unshadowed has
   none, and away from the cube the two look the same.
3. Turn the light off (only the environment lights the scene) and render:
   Illumination (Unshadowed) is black, Illumination (Total) is not.
   Give the cube a glowing (Luminous) material: it still adds nothing to
   Unshadowed.
4. Diffuse Shading (Unshadowed): Unshadowed tinted by the surfaces' colour.
5. Select the Light Path Expressions output: the Light Path Expression
   field, a Presets menu beside it, and "Type an expression" under it. On
   any other output these rows are greyed with the reason on hover.
6. Type `C<RD>L`: the line under it says "Exact" as you type. Type
   `C<RD>`: it says "Expected an end event (L, O, B or V) (at character
   6)", no dialog. Type `CDS.*`: "Not supported by the renderer: ...", and
   hovering the line names the nearest exact expression.
7. Presets: pick All Light, Direct Diffuse, Caustics; each fills the field.
   Give the light the LPE Label "Key": the Presets menu now also has All
   Lights and Light: Key.
8. Render with `C.*`: it looks like Final Color (before image processing).
   `CDS.*`: black, and the render window's status line says why.
9. Add a second light. Render > Add Per-Light Render Outputs (in the macOS
   menu bar): one "LPE: C.*L'<label>'" output per light; the unlabelled
   light's LPE Label is now its name. Run it again: the status line says
   every light already has one. Cmd+Z once: all the new outputs and the
   label go; Cmd+Shift+Z brings them back.
10. Render: each LPE: output shows only its light; with both, the light
    group pictures of the hidden `__lpe_` groups don't appear in the
    output picker.
11. Shaded AA Samples with Noise Threshold 0 (Render item): solid red and the
    status line says "Noise Threshold is off: every pixel gets every
    sample". Set a Noise Threshold (e.g. 0.05) and Maximum Quality 256: flat
    floor areas go blue/cyan, soft shadow edges and the cube's edges go
    towards yellow/red. Its Image Processing and Colorspace rows are greyed
    with the reason on hover.
12. Save Image > OpenEXR Multilayer: the Shaded AA Samples layer has R, G, B,
    A and Y channels (Y = the raw 0..1 share).
13. Save the scene with these outputs and an expression, reopen: all kept.
    An older build refuses the file (native version 55).
