# Texture Locators (stage 1): GUI checks

For Jason or Codex (Claude can't click Operandi). Use the QA build named in
the report (`Operandi texture-locators <commit>.app`), on a throwaway
scene with a plane that has UVs (e.g. a Unit Plane primitive), and any
picture file. Each line: do this → expect that.

1. Shading tab, Add Layer > Image Map → a new row; the Items list gains a
   "Texture Group" holding "Image Map (Texture)". Add a Constant → a second
   locator "Constant (Texture)" in the same group.
2. Click the small arrow after the Image Map row's type tag → a row with
   the locator's name appears under it. Click that row → Properties shows
   the "Texture Locator" sub-tab: Position, Rotation, Rotation Order, Size
   (1 m each), Auto Size, Projection Type UV Map, Projection Axis greyed,
   Horizontal/Vertical Repeat = Repeat, Repeat U/V 1, UV Map (none),
   UV Rotation 0, World Coordinates off, World Transforms on, then the
   stored-only settings (hover says so).
3. Pick a picture on the Layer tab. Render (Cycles) → the image follows
   the plane's UVs, as before this branch.
4. Projection Type Planar, Axis Z, then Render → one copy of the image per
   1 m, centred on the origin, +Y up. Projection Axis is no longer greyed.
5. Horizontal and Vertical Repeat → Reset, Render → one image in the
   middle; the rest of the plane shows the material below. Mirror → copies
   flip every other tile. Edge → border pixels stretch outward.
6. Size X = 2 → the image doubles in width. Drag Position X → the image
   slides; one Cmd+Z undoes the whole drag.
7. Move the plane item 1 m in X, Render → the image moves with it. Tick
   World Coordinates, Render → the image stays put; the plane slides under.
8. Auto Size → Position goes to the plane's centre, Size to its width and
   depth (the flat side keeps its value).
9. Projection Type Solid → the image disappears from renders and the
   viewport (the material below shows).
10. On the Layer tab, set the Constant layer's Locator to "Image Map
    (Texture)" → "Constant (Texture)" disappears from the Items list.
11. Delete the Image Map layer → its locator stays (the Constant still uses
    it). Delete the Constant too → the locator goes; the empty Texture
    Group stays. Cmd+Z brings layer and locator back together.
12. Save, reopen → layers, locators and settings come back. Open a scene
    saved before this branch that has an image layer → it gets a locator
    (UV Map) and renders as before.
