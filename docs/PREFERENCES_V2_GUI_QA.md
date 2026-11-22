# Preferences v2: GUI checklist

Spec `specifications/preferences-2026-11-06`. One pass in the QA build
(`~/Developer/Operandi-Builds/Operandi preferences-panel <commit>.app`),
Preferences with Cmd+,. Each change applies at once; quit and relaunch
once at the end to see it all kept.

## Tier 1

1. **Trackball.** Remapping > Trackball Rotation on. Alt-drag from the
   middle of the view straight up: the view goes over the top without
   stopping. Drag around near the edge: the view rolls. A view's
   Properties (O) > General Visibility > Trackball Rotation Off makes that
   view orbit the old way, unless Override Viewport-Level is on. Speed
   multiplier 3 turns faster.
2. **Keypad Enter.** Numeric Enter as Tab on; in a tool's number field,
   Fn+Return (keypad Enter) moves to the next field; Return still confirms.
3. **Dialogs under mouse.** On; open Set Material (or any dialog) with the
   pointer near a corner: it opens there, inside the window.
4. **Closest rollovers.** Selection > Selection Rollovers: Closest. In
   Vertex mode, hover an edge middle and a polygon middle: they
   pre-highlight too. Mouse Regions on: right-click an edge in Polygon
   mode switches to Edges with that edge selected.
5. **Point priority.** Tweak tool, hover just off a cube corner along an
   edge: with Point Selection Priority 0 the edge highlights, at 12 the
   corner.
6. **X-ray.** Transparency: Selection 40 %: select back polygons/points of
   a cube, orbit so they are behind: they show faintly. Tool Handles >
   Transparency: Handles 30 %: Move tool on a whole cube; the handles
   inside the cube fade (not during a drag).
7. **Display.** Grid Visibility off, then View > split or quad: new views
   have no grid. Flatness 100 %: the perspective flattens, the model keeps
   its size. Item Drawing: Current Scene 3×: cameras and lights draw three
   times bigger. Backdrop: Uniform Size on and Pixel Size 20 mm, then add
   a Backdrop: its panel shows both. Include Backdrops in Viewport Fitting
   off: Fit ignores a Backdrop.
8. **Tool Handles.** Large Points 18: Scale handle squares double. Small
   Points / Handle Scale: Small change falloff handles; Unselected Tool
   Handles Invisible hides them. Plane Handle Ratio 2: Move's centre
   square doubles. Rotate tool, hold Ctrl as you start dragging a ring: it
   steps by 15°.
9. **Tool Settings.** Default Action Center (component) Last Selected;
   pick Origin, relaunch: Origin again.
10. **Application.** Item Index Style Item_2: New Mesh twice gives
    "Mesh_2". Default Material "Metal": a new Box's polygons are tagged
    Metal; Set Material on an untagged polygon opens with Metal. Default
    Texture Map "UVs": a new Box's map is UVs. Default Scene: Choose… a
    saved scene; File > New starts from it, untitled. Use OS Color Picker:
    a light's colour swatch opens the macOS colour panel.
11. **Work plane.** Grid and Work Plane > Lock Preferred Plane with YZ and
    offset 1 m: the grid stands on YZ at X = 1 m in every view. Bias 50 %
    (unlocked) keeps the ground plane longer as you orbit. Increments
    Powers of 2: zooming steps the grid 0.5, 1, 2, 4.
12. **Colours.** Colors > Selection Rollovers: Catmull-Clark red; Shift+Tab
    a polygon to Catmull-Clark and hover it: red. Falloff Range Cool/Warm
    change the falloff weight dots.
13. **Reset.** Reset Panel on any page puts just that page back. Edit >
    Reset Preferences… asks, then resets everything (shortcuts kept).

## Tier 2

14. **Mesh Items.** New Catmull-Clark Display Level 4, Auto Create UV
    Map and Item Masks on: New Mesh has level 4, a UV map and its own
    shader tree group. Symmetric Primitive Creation off with Symmetry on:
    a Box isn't mirrored. Select Pasted off: Paste leaves the pasted
    polygons unselected. Auto Hide Child Items: hiding a parent hides its
    children with a plain click.
15. **Graphics memory.** (Max GL Polygon Count retired 2026-11-20.) A
    heavy scene (8M+ triangles) opens with no question and no Max GL row
    in Preferences > Display. Only a scene near the card's memory budget
    shows "Graphics memory full: n items drawn as boxes" and boxes its
    heaviest items.
16. **Rounding.** Coordinate Rounding Forced Fixed 0.25 m: drawing a Box
    on the ground snaps to quarter metres and the grid stays 0.25 m.
17. **Units display.** Color Unit System Hexadecimal: Set Material's
    colour reads like "cc". Factor or Range Float: Diffuse shows 0.8.
18. **Up Axis Z.** Orbit turns around Z, Top looks down Z, the ground grid
    lies on XY; import a Y-up OBJ and it stands upright.
19. **Input Device Tablet.** Drag the view header's Rotate button to the
    screen edge: it stops there (Mouse keeps going).
20. **File I/O.** Export OBJ in centimetres with Save Meshes as Groups:
    numbers ×100, `g` lines, a .mtl beside it. glTF Export Selected only
    writes the selected item.
21. **Rendering.** Render Threads 2 (automatic off): a final render uses
    two cores. Start Paused off: F8 opens the Preview already rendering.
    Default Image Format JPEG: renders save as .jpg.
22. **Overlays and UI.** Snapping State shows "Snapping: …". Popup
    Transparency 50 %: menus are see-through, dialogs aren't. Icon Size
    Large: bigger toolbox. Popover Dismissal Roll-Off: Viewport Properties
    closes when the pointer leaves.

## Colors page

- Open **Scheme**. Check the order Steel, Slate Blue, Fog, Classic Grey,
  Gradient Sky, Charcoal, Midnight, None More Black, then a divider and your
  saved schemes.
  Save a user scheme named Steel and confirm both entries appear and load
  their own colours. Edit a built-in and choose **Save to Scheme…**. The
  name field starts blank and saving creates a user scheme.
- Choose **Export…**, cancel, and confirm no file is written. Export to
  JSON, switch schemes, then choose **Import…** and pick the file. Check
  the cleaned filename in the dropdown, the loaded-colour count, and the
  restored colours after relaunch. Repeat with a bad hex value and an
  unknown key and check that the valid colours still load.
- In a fresh profile, verify that Steel is active and **Reset All** returns
  to Steel. In an upgraded profile, open a saved scheme that omitted some
  colours and check that its old appearance and selection survive relaunch.
  In a profile with individual edits, verify that the edited swatches keep
  their colours.
