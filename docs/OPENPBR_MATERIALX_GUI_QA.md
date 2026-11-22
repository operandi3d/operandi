# OpenPBR and MaterialX: GUI check

Branch `openpbr-materialx`. QA app: `Operandi openpbr <sha>.app` in
`~/Developer/Operandi-Builds`. Use the QA data folder, never your own
scenes. Test files: `~/Developer/Operandi-Content/Test-Library/ambientCG`
(.mtlx beside each texture set) and the OpenPBR examples (a checkout of
github.com/AcademySoftwareFoundation/OpenPBR, `examples/`).

Tick each line; note anything that looks wrong with a screenshot.

## Stage A: OpenPBR channels

1. [ ] Open an older scene with materials (any from before today). It looks
   the same in Preview/Render as in the main build.
2. [ ] Shading panel, select the Base Material. New rows appear: Coat Color,
   Coat IOR (1.6), Coat Darkening, Coat Anisotropy (after Clearcoat Bump);
   Sheen Color, Sheen Roughness (after Sheen Tint); Thin Film, Thin Film
   Thickness, Thin Film IOR (after Flatness); Transmission Scatter, Scatter
   Anisotropy, Thin Walled (after Absorption Distance); Albedo from
   Diffuse (off), Subsurface Albedo (after Subsurface Distance); Abbe
   Number (after Dispersion). Each has its status light and Reset.
   Thin Film Thickness shows µm.
2b. [ ] Greying (UIC-92): with a new material, Coat rows are greyed while
   Clearcoat Amount is 0 %, Sheen Color/Roughness while Sheen is 0 %,
   Thin Film Thickness/IOR while Thin Film is 0 %, Abbe Number while
   Dispersion is 0 %, Scatter Anisotropy while Transmission Scatter is
   black, Subsurface Albedo while Albedo from Diffuse is on. Hovering a
   greyed row says why; turning the control on un-greys it at once, and
   nothing moves.
2c. [ ] Older rows grey the same way (branch material-greying), live,
   in place, with the hover reason: Clearcoat Roughness and Clearcoat Bump
   while Clearcoat Amount is 0 % (and Match Specular on or Reflection
   Amount 0 %); Sheen Tint while Sheen is 0 %; Reflection Color, Amount and
   Fresnel while Match Specular is on, Reflection Fresnel while Reflection
   Amount is 0 %; Refraction Roughness, Absorption Distance, Transmission
   Scatter, Scatter Anisotropy, Exit Color, Dispersion and Abbe Number
   while Transparent Amount is 0 %; Subsurface Distance, Albedo from
   Diffuse, Subsurface Albedo, Subsurface Depth, Subsurface Phase and
   Subsurface Samples while Subsurface Amount is 0 %; Smoothing Angle while
   Smoothing is 0 %; Rounded Edge Angle / Round Same Surface Only while
   Rounded Edge Width is 0; Clip Value, Matte Color and GL Color while
   their switch is off. Each control sits above its rows; dragging the
   control off 0 un-greys them on the same frame.
2d. [ ] Texture keeps rows live: Clearcoat Amount 0 %, add a Constant
   layer in the material's group with Effect Clearcoat Amount: every
   coat row un-greys. Hide the layer (eye): they grey again. A Constant
   with Effect Refraction Roughness keeps that row live while Transparent
   Amount is 0 %; the other transparency rows stay greyed.
3. [ ] Clearcoat Amount 100 %, Coat Color red: Preview shows a red-tinted
   coat. Coat IOR 2.0 makes the coat reflection stronger.
4. [ ] Sheen 100 %, Sheen Color blue: a blue rim on a dark sphere.
5. [ ] Thin Film 100 %, Thickness 0.4: rainbow-tinted reflections.
6. [ ] Transparent Amount 100 %, Refractive Index 1.5, Dispersion 100 %:
   coloured fringes through the glass (needs a detailed environment).
7. [ ] Same glass, Absorption Distance 0.2 m, Transmission Scatter grey
   0.5: the inside goes milky.
8. [ ] Thin Walled on a glass sphere: no bending through it.
9. [ ] Subsurface Amount 100 %, Albedo from Diffuse off, Albedo
   orange: the scattering turns orange whatever the Diffuse Color.
10. [ ] Save, reopen: every new value comes back. Undo/redo each edit.

## Stage B: MaterialX

11. [ ] Select a shader tree layer, File > Import MaterialX…, pick
    `ambientCG/Bricks113A/Bricks113A_1K-JPG.mtlx`: a Bricks material
    (colour, roughness, normal, displacement) lands directly above the
    selected layer, in its group, and nothing is replaced; one Undo
    removes it all. With nothing selected it lands at the top of the tree.
12. [ ] Import `open_pbr_gold.mtlx`, `open_pbr_glass.mtlx`,
    `open_pbr_skin_iii.mtlx`, `open_pbr_carpaint.mtlx`: each renders
    plausibly. The status bar lists notes (e.g. "colours above 1 clamped").
13. [ ] A .mtlx with a node Operandi can't do (e.g. noise2d into roughness)
    shows "needs Node Material (not yet supported): <node>" and changes
    nothing (undo history unchanged).
14. [ ] Select the imported group, File > Export Material as
    MaterialX…: a .mtlx is written; the status lists anything left out.
    Import it again: same layers and values.
15. [ ] Add a Procedural layer to a group and export: the status names it
    as left out.
16. [ ] Preset Browser on `Test-Library/ambientCG`: each folder's material
    shows once, as a MaterialX entry ("X" in list view) named after the
    set, with its thumbnail. Double-click loads it, with the set's AO
    Multiply layer on top (MX-Q03).
17. [ ] Preset Browser gear: "Also write MaterialX for material presets"
    on, then save a material preset (Shading right-click > Save Preset):
    a .mtlx of the same name appears beside the .oppreset.
18. [ ] Command line: `operandi.materialx.import filename:<path>
    layer:<id>` and `operandi.materialx.export filename:<path>
    item:<name>` work; export with nothing selected and no item says
    "Select a material…"; `help operandi.materialx.import` shows the
    help line (no "placeholder").
19. [ ] Gear toggle on, File > Save Item Preset… with a material selected
    on the Shading tab: a .mtlx appears beside the .oppreset; toggle off:
    none. `preset.saveAs kind:material filename:<path> mtlx:true` writes
    one with the toggle off.
