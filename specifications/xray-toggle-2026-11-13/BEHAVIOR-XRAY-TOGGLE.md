# X-Ray / Ghost quick toggle — behavior spec v1

Provenance: the user guide's 3D Viewport page (Transparency Overrides
buttons) and 3D Viewport Properties page (Transparency Mode and its
colours); asked for by Jason (2026-11-14) after the see-through-material
problem in the Whiskey Glass tutorial. Not yet measured through the Modo
bridge. REA-assisted: no.

## What Operandi already has

The **Transparency Mode** viewport property (Normal, Ghost, X-Ray) with its
front/back colours and opacities, the overlay mode and flat shading. It is
drawn correctly, but it is only reachable inside Viewport Properties, so in
practice nobody finds it. There is also the selection see-through
preference (selection highlighting shown through surfaces).

Gap: no quick way to switch it, and no way to set it separately for
inactive meshes from the view.

## What the reference has (from the guide)

- **Three toggle buttons in the 3D view's own button row**: Normal, Ghost,
  X-Ray (Transparency Overrides).
  - **Click** sets the mode for **active** meshes.
  - **Alt+click** sets it for **inactive** meshes.
- Modes:
  - **Normal**: overrides off (the shading style as set).
  - **Ghost**: front-facing polygons semi-opaque with a specular
    highlight; back-facing polygons fully opaque.
  - **X-Ray**: front-facing polygons transparent with a specular
    highlight; back-facing polygons fully opaque. The guide's use case:
    seeing and selecting things inside a mesh (a rigged character's joints).
- Not available in Wireframe style. For inactive meshes it only shows when
  the inactive shading style isn't Wireframe.
- Colours and opacities per mode, for front and back faces, are the
  existing Transparency Override properties: Polygon Color (front),
  Polygon Opacity (front, Ghost), Wireframe Color (front), and the same
  three for back faces. They are set separately for active and inactive
  meshes.

## Required in Operandi

1. Three buttons (Normal / Ghost / X-Ray) in each 3D view's button row,
   showing the active-mesh mode as pressed; Alt+click sets the
   inactive-mesh mode. Greyed with a tip in Wireframe style.
2. The same three as commands, so they can go in the Input Map and a pie.
   For a one-key toggle like the one in another popular open-source 3D app,
   a command that toggles active meshes between Normal and X-Ray, with no
   default key unless the default-shortcut spec gives one (Q1).
3. Per view, like the other viewport properties; saved with the layout
   like the rest of them.
4. Picking while in X-Ray: see Q2. Until it is measured, clicks pick as
   today (nearest surface under the pointer); the middle-button lasso
   already selects hidden elements.

## Acceptance (by eye)

- XR-01 Cube in front of a sphere, active, Advanced style: click X-Ray →
  the cube's front faces vanish to a highlight, its back faces stay
  opaque, and the sphere shows through the front.
- XR-02 Alt+click Ghost with the sphere inactive → the sphere goes
  semi-opaque; the active cube is unchanged.
- XR-03 Wireframe style → buttons greyed.
- XR-04 The toggle command, run twice → Normal, X-Ray, Normal.

## Open questions

- Q1 Default key for the toggle (none known in the reference's key map).
- Q2 Whether clicks in X-Ray pick through the transparent front faces to
  what is behind them.
- Q3 Whether Ghost/X-Ray also apply in the textured/material styles or only
  the solid/advanced ones.
