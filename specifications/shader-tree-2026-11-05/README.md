# Shader Tree (skeleton)

**Status: hand-off v0.2, 2026-11-05.** v0.2: evaluation rules in §8 measured in the reference app (layer mask direction, textures-only groups, several group masks, group blend/opacity on all channels, topmost shader); §10 panel conventions and Gradient/Variation defaults. Earlier: (v0.1 adds four provisional structural points; see the top of the behaviour file.) For building the shader tree's data
model and panel. Nothing needs to render yet. Behaviour only, in our own
words. REA-assisted: no.

Sources: the public Modo 17.1 user guide (`[D]`), and the default new-scene
tree and panel header as seen in Modo 17.1v1 (`[S]`, described in words).
Anything we guessed is `[?]` and listed in QUESTIONS.md with the placeholder
to build.

## Files

- `BEHAVIOR-SHADER-TREE.md`: item types, default scene, panel layout and
  columns, add / drag / parent rules, channels per item, effect and blend
  menus, a provisional evaluation model, suggested build slices.
- `QUESTIONS.md`: open items with the placeholder to use for each.

## Decided around this work

- Rendering goes through Cycles. The tree is the front end and will later be
  compiled into Cycles shaders. Not part of this slice.
- Materials use Principled-style channels with friendly names.

## Out of scope for the skeleton

Rendering and evaluation, node graphs, fur/hair/volumetrics/particles,
baking, texture cache, presets and the Library contents, instances and
references, Materials view details, Find.

## Suggested first slice (ST-1)

Data model + default scene + panel with its five columns, Add Layer for a
few types, rename, eye toggle, blend and effect menus, opacity drag,
delete, drag to reorder and to parent, undo, save/load. See §9 of the
behaviour file for the following slices.
