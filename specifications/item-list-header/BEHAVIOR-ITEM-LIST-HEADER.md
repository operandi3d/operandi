# Items list header: Add Item, Select, Filter (behaviour spec v1)

2026-11-06. Behaviour only. Sources: public user guide (Items List page), plain-text UI configuration, scripted state queries and a hands-on look at the live menus in the reference app 17.1. Nothing reverse engineered. Where something was not verified it says so.

## IL-00 Header row layout (left to right)

| # | Control | Kind | Tooltip (paraphrased) |
|---|---|---|---|
| 1 | **Add Item** + **▾** | split button | "Add new items to the scene." |
| 2 | **Select** | text button, opens a popover | "Select items in the tree by a name pattern." |
| 3 | **Filter** | text button, opens a popover | "Sort and filter items by item type." |
| 4 | Show-only-selected | icon toggle, default off | "Show only selected items." |
| 5 | Search-notes | icon toggle, default off | "Only search the notes for an item." |
| 6 | Text filter | icon toggle (funnel), default off | "Text filter." Turns on a search row (IL-07). |

The header sits directly under the Items/Shading tab row and above the column titles. There is no "Item" button in the reference app.

## IL-01 Add Item split button

- **▾ arrow**: opens the nested Add Item menu (IL-02).
- **Label part**: in the reference app it opens an "Add Item" item browser (a floating, pinnable panel with a directory tree on the left: Items > Other, Deformers, Dynamics, Falloffs, Forces, Lights, Locators, Particles, Procedural, Volumes; a "Search All" text field; a list of item types with icons on the right). **Operandi now:** the label opens the same nested menu as the arrow. The browser is a later item (needs a preset-browser style panel Operandi doesn't have).

## IL-02 Add Item menu (observed in 17.1, exact order)

Top of the menu is a blank text row. It looks like a type-to-find field; not verified (typing into the app could not be automated). Operandi: make it a find field that filters the menu as you type (sane, cheap).

```
[find field]
Recent            ▸   most recent first (see IL-04)
Backdrop Item
Camera
Mesh
──────────
Deformers         ▸   Bend Effector, Curve Constraint Effector, Lag Effector,
                      Lattice Effector, Magnet Effector, Mesh Wrap Effector,
                      Morph Container, Morph Influence, Soft Lag, Twist Effector,
                      Weight Container
Dynamics          ▸   Collision Emitter, Dynamic Replicator, Procedural Shatter,
                      Constraints ▸ (Constraint Modifier, Hinge, Pin, Point,
                                     Slide Hinge, Spring)
Falloffs          ▸   Area, Bezier, Blend, Capsule, Curve, Falloff Operator,
                      Incidence, Linear, Radial, Spline, Texture, Weight Map
                      (each labelled "<name> Falloff" except Falloff Operator)
Forces            ▸   Curve, Drag, Linear, Newton, Radial, Turbulence, Vortex, Wind
                      (each "<name> Force")
Lights            ▸   Area Light, Cylinder Light, Directional Light, Dome Light,
                      Mesh Light, Photometric Light, Point Light, Portal, Spot Light
Locators          ▸   Group Locator, Locator, Texture Locator
Particles         ▸   Replicator,
                      Modifiers ▸ (Particle Audio / Expression / Look At / Modifier /
                                   Random / Sieve / Snap / Step / UVT Modifier)
                      Point Clouds ▸ (CSV Point Cache, Curve Particle Generator,
                                   Particle Cloud, Particle Generator, Realflow Particles,
                                   Stitch Particle Generator, Surface Particle Generator)
                      Simulation ▸ (Collector / Emitter, Curve Emitter, Dynamic Collider,
                                   Dynamic Fluid, Particle Flocking, Particle Operator,
                                   Particle Simulation, Radial Emitter, Source Emitter,
                                   Surface Emitter, Terminator)
Procedural        ▸   Deferred Mesh, Fractal Gasket, Lace Item, Poisson Particles,
                      Proxy, Sphere, SubDFusion
Volumes           ▸   Blob, Ground Plane, Render Boolean, Sprite, VDBVoxel, Volume
Other             ▸   Bezier Node, Canvas, Channel Handle
```

Entries inside each submenu are alphabetical. The three top-level types (Backdrop Item, Camera, Mesh) are alphabetical too. Categories are alphabetical with Other last.

## IL-03 What picking an entry does (observed with Locator)

1. Creates one new item of that type with factory defaults, at the world origin.
2. Name = type label; a clash gets " (2)", " (3)" … (Locator, Locator (2)).
3. The new item becomes the only selected item.
4. Placement in the list: directly **after the selected item**, as a sibling (observed with a root-level selection, so it went to root). With nothing selected it landed after the previously added item. **Operandi:** nothing selected → append at the end of the root list (sane rule).
5. One undo step removes it.
6. The type goes to the top of Recent (IL-04).

## IL-04 Recent submenu

- Lists item types added recently from Add Item, newest first; adding a type already in the list moves it to the top (observed: Dome Light, Spot Light, Backdrop Item, then a Locator add put Locator first).
- Length cap not observed. Operandi: keep 8, remember across restarts.
- Empty list: show one disabled "(none)" row.

## IL-05 Select button: "Item Patterns" popover

Rows top to bottom:
1. **Pattern** text field, default `*`. Wildcard `*` = any run of characters. Matching is case-insensitive on the item name (the guide's example `cam*` picks "Camera"). Operandi: also allow `?` = one character.
2. **All / None / Invert** buttons: tick all, untick all, flip the type ticks below.
3. **Limit to current selection child items** checkbox, default off: when on, only items parented (at any depth) under the currently selected items are considered.
4. Type checkboxes, **all ticked by default**: Backdrops, Cameras, Deformers, Dynamics, Forces, Groups, Lights, Locators, Meshes, Mesh Instances, Particles, Replicators, Texture Locators, Volumes. Then **By Color** (default off) and a **Color** dropdown (default "(none)"); when By Color is on, only items with that editor colour match.
5. **Action** section (collapsible, open by default), four buttons:
   - **Select**: select every matching item, replacing the current selection.
   - **Unselect**: remove matching items from the current selection.
   - **Group**: make a new group containing the matching items.
   - **Hide**: hide the matching items (visibility off).
- Whether settings persist was not checked. Operandi: remember them for the session.
- Each action is one undo step.

## IL-06 Filter button: "Item List Filters" popover

Changes only what the list **shows**; it never changes selection or the scene.
1. **Flatten / Sort by Name / Sort by Type**: toggle buttons, all off by default.
   - Flatten: no tree; every item at the base level, nesting shown by one dot per level in front of the name.
   - Sort by Name: alphabetical (within each parent unless flattened).
   - Sort by Type: grouped by item type.
2. **All / None / Invert** for the type ticks.
3. Type checkboxes (default in brackets): Backdrops ✓, Cameras ✓, Deformers ✓, Dynamics ✓, Forces ✓, Groups ✓, Lights ✓, Locators ✓, Meshes ✓, Mesh Instances ✓, Particles ✓, Replicators ✓, **Texture Locators ✗**, Volumes ✓, **Has Notes ✗**, **By Color ✗**, then a **Color** dropdown "(none)". Unticking a type hides all items of that type from the list. Has Notes on = show only items that have notes. By Color on = show only items with the chosen editor colour.
- Hidden-by-filter children of a visible parent are simply not drawn; a visible child of a filtered-out parent: not verified. Operandi: keep the parent row (greyed) so the tree still makes sense.
- Persistence not checked. Texture Locators off is probably the factory default (not confirmed on a clean install). Operandi: save filter state with the layout.

## IL-07 Text filter (funnel toggle)

- On: a **Search** row appears under the header: a text field filling the width and a **Flat** toggle at the right end. Off: the row goes away and the filter clears.
- Typing narrows the list to items whose names contain the text (not verified by typing; guide + layout). Flat shows matches as a flat list instead of with their parents. With the Search-notes toggle on, the text is matched against item notes instead of names.

## IL-08 Show-only-selected toggle

- On: the list shows only the selected items (plus, Operandi sane rule, their parents greyed so position in the tree is clear). Off by default.

## Operandi support (Add Item entries)

| Entry | Now | Later | Notes |
|---|---|---|---|
| Mesh | ✅ | | existing New Mesh |
| Camera | ✅ | | |
| Backdrop Item | ✅ | | |
| Lights: Directional, Point, Spot, Area | ✅ | | in main |
| Lights: Cylinder, Dome, Mesh Light, Photometric, Portal | ✅ once the light-properties work merges | | all 9 exist on that branch |
| Locators: Locator, Group Locator | | **next** (small) | needs an empty "locator" item kind; Group Locator = same with a folder icon. Unlocks "Parent to Group Locator" too |
| Locators: Texture Locator | | with texture layers | |
| Falloffs | | later | Operandi falloffs are tool settings, not items |
| Deformers | | later (deformer stack) | |
| Procedural (Deferred Mesh, Fractal Gasket, Lace, Poisson, Proxy, Sphere, SubDFusion) | | later | some tie into Mesh Fusion / procedural work |
| Volumes (Ground Plane, Render Boolean, Blob, Sprite, VDB, Volume) | | later | Ground Plane is the easiest |
| Dynamics, Forces, Particles | | much later | no simulation system |
| Other (Bezier Node, Canvas, Channel Handle) | | much later | rigging/UI items |

**Recommendation for the builder:** show the full menu structure, with not-yet-supported entries **disabled with a "Not yet in Operandi" tooltip** (same pattern as other "Missing" menu items), and hide whole categories that have nothing enabled behind a "Show unsupported" option off by default. That keeps the menu honest and short today and the layout ready as types arrive.

Filter / Select type checkboxes: show only the types Operandi has (Backdrops, Cameras, Groups once item groups merge, Lights, Locators once they exist, Meshes). Add the rest as their item kinds land.

## The current Operandi "Item" button

Today the Items list header has **Add Item** (New Mesh, New Mesh Beside Selected, Backdrop Item, Camera, Light ▸) and a second **Item** menu with Create Backdrop, Create Camera, Create Light ▸, Delete Item, Rename…

- The three Create entries duplicate Add Item.
- Delete Item and Rename… are already on the row right-click menu, the Delete key and double-click rename.
- **Recommendation: drop the Item button.** Nothing is lost. Put "New Mesh Beside Selected" behaviour into the default placement rule instead (IL-03 point 4: new items go beside the selection), so the extra entry isn't needed either.

## Open questions

- IL-Q1: The top blank row of the Add Item menu: confirm it's a find field (Jason, 5 seconds: open the menu and type "spot").
- IL-Q2: Should Add Item's label open the menu (recommended now) or wait for a browser?
- IL-Q3: Disabled-unsupported entries vs hiding them entirely (recommended: disabled, with whole empty categories hidden).
