# Light properties: hand-off (2026-11-06)

Behaviour-only spec for every row on the light-specific Properties tab of all
nine light kinds (Area, Cylinder, Directional, Dome, Mesh, Photometric, Point,
Portal, Spot) and of the light material. Clean-room: built from the public
user guide, the shipped plain-text panel-layout and item-help config files,
and one read-only scripted readback of a freshly created light of each kind.
No reference-app images or internals. REA-assisted: no.

Files:
- `BEHAVIOR-LIGHT-PROPERTIES.md`: panel shape, shared rows (LP-1x/2x), Light
  Linking (LP-3x), Volumetrics (LP-4x), per-kind rows (LP-DIR, LP-SUN, LP-PT,
  LP-SP, LP-AR, LP-CY, LP-DM, LP-MS, LP-PH, LP-PO), light material (LM-).
- `QUESTIONS.md`: open points, each with a pick to build on meanwhile.
- Display, Assembly, User Channels, Tags: **not repeated here**. Same on every
  item kind; see `../camera-properties-2026-11-06/BEHAVIOR-ITEM-COMMON-TABS.md`.

Builds on `cameras-lights-2026-11-04` (CL-xx): the maths of brightness, cone,
spread, area size, facing axis and Cycles mapping is there.

## The four shared tabs, in plain English

- **Display**: how the item is *drawn in the viewport*, never how it
  renders. Show/hide it, make its icon bigger (Size), put a text label next
  to it, give it a custom wire colour. The "Add ..." buttons bolt on extra
  viewport gadgets (rulers, sticky notes, a dial, a motion trail).
- **Assembly**: "make it foolproof" settings. *Select* and *Lock* stop the
  item being clicked or moved by accident (handy for a light you've aimed
  perfectly). The rest is for rig builders: redirect a click to another
  item, arrow-key jumps between items, tooltips, what happens when it's
  dragged onto something. For a modeller only Select and Lock matter.
- **User Channels**: your own *custom settings* bolted onto the item, e.g.
  a "Door Open" slider. On their own they do nothing; they exist to drive
  other settings via links/expressions. Needs a rigging system to be useful.
- **Tags**: free-form *labels* stored on the item (key = value) for scripts
  and pipelines to find things. No visual effect.

## Build tiers

Light kinds Cycles supports natively: point, spot, sun (directional), area
(rectangle/disk/ellipse), background. Portal = a Cycles area-light flag. IES
files = a Cycles texture node on point/spot. Light linking = native in
current Cycles.

- **Tier 0, already there (verify only):** Name, Transform, intensity,
  Light Color, Falloff Type, Shadow Type None/Ray Trace, Spread Angle,
  Point/Spot Radius, Cone Angle, Soft Edge, Area Shape + Width/Height,
  light material Enable/Color, Display Size.
- **Tier 1, small and worth it now:** Render (Default/Yes/No), Dissolve,
  Visible to Camera / Reflection / Refraction (Cycles ray visibility),
  Affect Diffuse / Specular as on/off switches, Set Temperature (Kelvin to
  colour), **Portal** as a new kind (window helper; big noise win for
  interiors).
- **Tier 2, nice to have:** Light Linking (needs item groups first),
  **Cylinder Light**, **Mesh Light** (emissive mesh), **Photometric Light**
  (IES file), **Physical Sun** (place + date + time aims the sun; pairs with
  the environment's daylight sky), Targeting (Set Target, shared with
  camera).
- **Tier 3, later:** Volumetrics (visible beams, needs scene fog), LPE Label
  (per-light render passes), Importance, **Dome Light**, light-material
  layer stack (gobo textures), Blend Mode / Opacity, Affect Subsurface,
  Sun Gamma, Store Location.
- **Never:** Deep Shadow Map + Shadow Map Res, Shadow/Photon Size, per-light
  Samples (Cycles has none), Affect Caustics, Shadow Color, Invert, the
  Octane Light section.
