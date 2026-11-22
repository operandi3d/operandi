# Preferences: Mesh and tool defaults (behaviour spec v1, 2026-11-04)

Behaviour-only. "Not yet measured" = factory value still to be read (v2);
use a provisional value and log it. Settings apply at once and persist per
user.

## Mesh Items (Defaults > Mesh Items)

| Setting | Type | Default | Behaviour |
|---|---|---|---|
| Auto Create Item Transforms | checkbox | not yet measured | New items get position, rotation and scale channels at once instead of when first needed. |
| Auto Hide Child Items | checkbox | not yet measured | Hiding or showing a parent also hides or shows its children. |
| Auto Create UV Map | checkbox | not yet measured | New mesh items get an empty UV map. |
| Save Vertex Maps Selection to Scene | checkbox | not yet measured | Which vertex maps are selected is saved and restored with the scene; when off, none are selected on reopen. |
| New Patch Display Level | whole number | not yet measured | Subdivision display level given to new meshes (editable per item). |
| New Catmull-Clark Display Level | whole number | not yet measured | Catmull-Clark level given to new meshes (editable per item). |
| Default Primitive Polygon | dropdown (polygon types) | not yet measured | Starting value of the polygon-type option in primitive tools (e.g. Sphere, Torus). |
| Symmetric Primitive Creation | checkbox | not yet measured | With symmetry on, a new primitive is also created mirrored across the symmetry plane. |
| Flatness Limit (3D Editing) | number | not yet measured | Threshold above which a polygon counts as non-planar (used when finding non-planar polygons). Higher = fewer flagged. |
| Deselect Elements after Copying | checkbox | not yet measured | Copy clears the selection. |
| Select Pasted Elements | checkbox | not yet measured | Pasted elements become selected. |
| Deselect Elements before Pasting | checkbox | not yet measured | Paste clears the existing selection first. (By default the old selection stays and the pasted elements are not selected.) |

## Names (Defaults > Application)

| Setting | Type | Default | Behaviour |
|---|---|---|---|
| Default Material | text | not yet measured | Material tag given to new polygons; prefilled when setting a material. |
| Default Texture Map | text | not yet measured | Name of new and auto-created UV maps. |
| Item Index Style | dropdown: Item (2), Item(2), Item 2, Item_2, Item2 | not yet measured | How a number is appended when a new item, layer or map name is already taken. |

## Tool defaults (Defaults > Tool Settings)

| Setting | Type | Default | Behaviour |
|---|---|---|---|
| Default Action Center (component modes) | dropdown: action centres + Last Selected | not yet measured | Action centre active at startup in point/edge/polygon modes. |
| Default Action Axis (component modes) | dropdown: action axes + Last Selected | not yet measured | Action axis at startup in component modes. |
| Default Action Center (item mode) | as above | not yet measured | Same, item mode. |
| Default Action Axis (item mode) | as above | not yet measured | Same, item mode. |
| Auto Activate | checkbox | not yet measured | Makes each tool's Auto Activate option one global setting. |
| Select Through | checkbox | not yet measured | Makes each tool's Select Through option one global setting. |

Not for now: brush and constraint defaults, curve display settings,
item masks, instance bounding boxes, Slip UV startup state.

Open: MTD-Q1 factory values and dropdown choices.
