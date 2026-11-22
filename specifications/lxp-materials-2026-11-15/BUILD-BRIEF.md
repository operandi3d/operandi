# Build brief: Modo material presets (.lxp) in the preset browser

Jason approved this on 2026-11-14 at 22:08 CT ("make it happen"), with all the LXP-Q1..Q8 defaults in `BEHAVIOR-LXP-MATERIALS.md` (same folder). That file is the behaviour spec. It is behaviour only and safe for builders.

Clean room: the reader uses only the published LXO container and chunk layout (FORM, DESC, PRVW/THUM, ITEM with channels by name, GRAD gradient values). Unknown records are skipped by size, or the preset is refused (Q-MC1). Never inspect Modo file bytes to work out gaps. Test only with presets the user points at; ship none.

## Stage A: gradient gaps (spec section 7.4, items 1–3)
Do this first or alongside stage B, since it is what makes about a third of presets convert properly.
1. Gradient **Incidence** input: check it works on Diffuse Amount, Diffuse Color, Roughness, Specular/Reflection Color and **Group Mask** effects, in the viewport and in Cycles.
2. **Driver A–D**: a layer whose effect is Driver A–D feeds a gradient that uses that driver as its input.
3. **Process** (hue, saturation, value, bias, gain) and **Occlusion** texture layers, if they don't exist yet. Occlusion uses Cycles AO.
These are our own features. Check them against the existing gradient and procedural specs.

## Stage B: .lxp material reader and converter
1. In the browser, a material .lxp (DESC names a material type) becomes a normal Materials entry with Modo's PRVW thumbnail (LXP-01..05).
2. Read mask, material (advancedMaterial / unrealShader / unityShader), texture layers, image clips and texture locators with the existing LXO item and channel reader.
3. Convert per spec sections 3, 4 and 7: channel table 3.1; "metal made by hand" → metalness (3.2); legacy models (3.3); Unreal/Unity (3.4); effects (4.1); layer settings, colour space, image path resolution (4.2); procedural 1:1 mapping (7.2); drops (7.3).
4. Drop behaves like any library material, in one undo step. Write one import note listing everything dropped or approximated, and add a "partial" badge per LXP-05.
5. Never fake a dropped plug-in layer (LXP-Q7).

## Tests
- A unit test per channel rule in 3.1–3.4, from hand-made values (e.g. specAmt 0.04 gives IOR 1.5; useRefIdx on ignores specAmt; diffAmt 0 with specAmt 1 and a coloured spec gives metal).
- Effect id table round trip (4.1).
- Colour space rule: "(default)" on a roughness map is read linear.
- If the user's content folder is present on the test machine, an opt-in integration test loads 10 presets and checks: no crash, a material created, the note lists any dropped layers. Never commit those presets.
- GUI QA: Polished Gold looks metallic gold, glass_crown is clear glass, White LED glows, and the Unreal Bowling Ball shows its image maps.

## Open items the builder decides (log them as builder questions)
- Exact "partial" badge visuals (follow the bible).
- Whether an Operandi gradient key type has no Modo equivalent.
