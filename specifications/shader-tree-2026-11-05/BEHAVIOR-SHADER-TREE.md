# Shader Tree: behaviour for the skeleton build

Hand-off v0.2, 2026-11-05 (v0.2: §8 evaluation rules now **confirmed in the reference app** by measured renders; §10 adds panel conventions and Gradient / Variation defaults. v0.1: item mask may name a group, "(none)" tag, mask nesting, Library guard). Behaviour only, in our own words. REA-assisted: no.
Goal of this slice: **the data model and the panel in place**. Nothing has to
render yet. Shading maths is described in §8 only so the data model has
room for it; it is provisional.

Source tags:

- `[D]` stated by the public user guide (paraphrased)
- `[S]` seen on screen in Modo 17.1v1 (default new scene, described in words)
- `[?]` our best reading or a placeholder; must be confirmed (see QUESTIONS.md)
- `[M]` measured in the reference app (17.1v1) from test renders of unlit
  surfaces; numbers below are the measured pixels

---

## 1. What the shader tree is

- One tree per scene that holds **all surfacing, shading and render-output
  items**. It lives in its own panel ("Shading" tab), separate from the item
  list. `[D]`
- Surfacing is separate from geometry. Every layer affects the **whole
  scene** unless a container above it limits it with a **mask** (by item,
  or by polygon tag: Material, Part, Selection Set). `[D]`
- Layers are evaluated **bottom to top**, like image-editor layers. A higher
  layer hides lower ones of the same kind unless it is masked, partly
  transparent, or uses a non-Normal blend mode. `[D]`

## 2. Item types

### 2.1 Tree roots and sections (top level of the panel)

Top-level rows, in this order `[S]`:

| Row | What it holds | Notes |
|---|---|---|
| **Render** | The main tree: outputs, shaders, materials, groups, textures. | The "trunk". Expandable. Selecting it shows render settings (later). |
| **Library** | Shared material groups that can be referenced elsewhere. | Folder-style row. Empty in a new scene. `[S]` |
| **Nodes** | Shading items used only by node graphs. | Folder-style row. Empty in a new scene. Out of scope beyond the row. |
| **Lights** | One entry per light item; each light carries its own light material and optional texture layers. | Expandable. Mirrors the scene's lights. |
| **Environments** | Environment items, each with an environment material and optional texture layers. | Expandable. |
| **Bake Items** | Baking setups. | Empty row for now. |
| **FX** | Effect items (e.g. sprites/volumes). | Empty row for now. |

For the skeleton, Library/Nodes/Bake Items/FX can be fixed, empty,
non-editable section rows. Lights and Environments should list what the
scene has (cameras-lights work already creates a light). `[?]` whether a
default environment exists is Q-1b.

### 2.2 Items that live under Render

| Type | Role | Can contain children | Effect column shows |
|---|---|---|---|
| **Render Output** | One output image (Final Color, Alpha, Depth…). | No | Output kind, e.g. "Final Color", "Alpha" `[S]` |
| **Shader** | Visibility / shading-rate / lighting controls for what it covers. | No | Shading kind, default "Full Shading" `[S]` |
| **Material** | The surface model: many channels at once (diffuse, specular, roughness…). | No (but see texture masks, §5.4) | The mask it ends up under; a top-level material shows "(all)" `[S]` `[?]` |
| **Material Group** (menu name "Group") | Container + mask + blend. | Yes | `[?]` probably its mask summary |
| **Folder** | Pure organiser. No properties. | Yes | blank `[?]` |
| **Texture layer** (Image Map, Constant, Gradient, procedurals, Vertex Map, Process…) | Drives **one** channel, chosen by its Effect. | Only its own layer mask (§5.4) | Effect name, default "Diffuse Color" `[D]` |

Every texture layer also owns a **Texture Locator** (a separate scene item,
shown in the item list inside a "Texture Group" folder) that holds its
projection and placement. For the skeleton: create the locator item and link
it; projection channels can be stubs. `[D]`

## 3. Default new scene

Under Render, top to bottom `[S]`:

1. **Alpha Output** (Render Output, effect Alpha, 100%)
2. **Final Color Output** (Render Output, effect Final Color, 100%)
3. **Base Shader** (Shader, effect Full Shading, 100%)
4. **Base Material** (Material, effect column "(all)", 100%)

All four enabled (eye on). Blend column empty for all four. Render is
expanded; the other sections are collapsed. `[S]`

Base Shader and Base Material are not inside any group, so they cover the
whole scene. `[D]`

## 4. The panel

### 4.1 Header controls `[S]`

- Row 1: **View** dropdown (values: Shader Tree, Materials; default Shader
  Tree), **Assign Material** button, a filter-toggle icon at the far right.
- Row 2: **Filter** dropdown (default "(none)"), **Add Layer** dropdown
  button, a small collapse/back icon at the far right.

Filter values `[D]`: (none), Material, Texture, Group, Shader, Render Output,
Fur Materials, Filter on Selection. Filtering only hides rows; nothing is
disabled.

Materials view `[D]`: shows only the material tags used by the selected
items, alphabetically, each expandable to its material; hides environments,
shaders and outputs. Skeleton: may be a stub that shows the same tree.

Find `[D]`: a search bar (substring, or pattern mode with `*`, `?`, `|`,
`[]`, negation), Match Case, Flat (no indentation). Later.

### 4.2 Columns `[S]`

| Column | Header | Content |
|---|---|---|
| Visibility | eye icon | Eye per row = the item's **Enable**. Click toggles; drag across rows toggles many. `[D]` |
| Name | "Name" | Tree with indentation, expand triangle on rows with children, type icon, name. Double-click renames inline. `[D]` |
| Blend | "B" | Blend mode; click opens a menu of modes (§7). Empty when Normal `[S]` `[?]`. |
| Opacity | "%" | Opacity as a number 0–100; drag the number to change it. `[D]` |
| Effect | "Effect" | Effect / kind name with a dropdown arrow; click or right-click opens the effect menu for that item type. `[D]` `[S]` |

Rows with a layer mask show a small "+" before the name that expands to show
the mask; Render itself shows a "+" too. `[D]` `[S]`

### 4.3 Selection and editing `[D]`

- Click selects; Ctrl/Cmd+click adds/removes; Shift+click selects a range.
- Selecting rows shows their properties in the Properties panel. Several
  rows of the **same type** gang-edit.
- Right-click on a row: Rename, Editor Color (12 row tints), Select Polygons
  (of that row's mask), Duplicate, Create Instance, Copy, Paste, Delete,
  Change Type, Save Preset, (bake commands, later).
- Delete key removes the selected layers.
- Shift+click on an expand triangle expands/collapses the whole branch.
- Ctrl/Cmd+G puts the selected rows into a new **untagged** Material Group.

## 5. Order and parenting rules

### 5.1 Adding layers `[D]`

- Add Layer with nothing selected, or a non-group selected at the top
  level: the new layer is inserted **at the top of Render's children,
  directly below the Base Shader**. `[D]` (Q-2 covers other selections.)
- With a Material Group selected: the new layer goes **inside that group,
  as its first child**.
- Folder: added at the top level of the nearest enclosing group.
- Add Layer → Render Output (or one of its shortcuts): added **at the top of
  the tree** so it sees everything.
- Adding a texture layer also creates its Texture Locator.

Add Layer menu groups `[D]` (for the stub menu): Special (Folder, Group,
Shader, Render Output, Material Preset, Surface Generator…), Material,
Custom Materials, Image Map, Processing (Constant, Gradient, Process,
Vertex Map Texture, Variation, Occlusion…), Textures (procedurals),
Render Outputs. Exact order: Q-24.

### 5.2 Drag and drop `[D]`

- Drag a row to reorder; an insertion line shows where it lands.
- Drop **onto a closed group**, or between two children of an open group,
  to parent it there. The row now inherits the group's mask.
- Drop a **texture layer onto another texture layer**: the dropped one
  becomes the target's **layer mask** (its effect becomes Layer Mask; it
  appears under the target's "+").

### 5.3 Rules that order carries `[D]`

These matter to the future evaluator; the skeleton only needs to preserve
order exactly.

- Higher rows override lower rows of the same channel.
- A texture with effect **Layer Mask** sits directly **below** the one layer
  it masks (that layer is directly above it). `[D]` (the docs agree on this
  for surfaces; Q-9)
- A **Group Mask** texture inside a group masks the whole group.
- A **Gradient** that reads other layers needs them **below** it.
- A **Process** layer adjusts the texture layer **below** it.
- A **Texture Offset** layer distorts the layers **above** it in the same
  group.
- An extra Shader has to sit **above** the Base Shader (inside a masked
  group) to win over it. `[D]`

### 5.4 What can parent what `[D]` `[?]`

- Render: any item type.
- Material Group / Folder: any item type, including groups (nesting
  unlimited).
- Texture layer: only its layer-mask texture(s). A mask texture may itself
  have a layer mask (nesting). `[?]`
- A group may hold **several** Group Mask layers; store them like any
  other layer, in order. `[?]`
- Material, Shader, Render Output: no children. `[?]` (Q-42)
- Lights and Environments sections: their own material + texture layers
  only.
- Library: dropping a group that has a polygon tag or item mask into the
  Library is refused (or warns and offers to wrap the layers in an untagged
  group first). `[?]` (Q-44)

## 6. Channels per item (skeleton set)

Only what the data model and Properties panel need now. Defaults marked `[?]`
are placeholders.

### 6.1 Common "Layer" block (almost every item) `[D]`

| Channel | Type | Default |
|---|---|---|
| Enable | bool | on |
| Invert | bool | off |
| Blend Mode | enum (§7) | Normal |
| Opacity | percent 0–100 | 100% |

Materials and Shaders also have **GL Display** (bool, show textures in the
GL viewport; default on `[?]`).

### 6.2 Material Group `[D]`

| Channel | Type | Default |
|---|---|---|
| Layer block | §6.1 | |
| Item | reference to a mesh item **or a group of items** (all members), or "(all)" `[?]` | (all) |
| Polygon Tag Type | enum: Material, Part, Selection Set | Material `[?]` |
| Polygon Tag | "(all)", "(none)" (covers nothing), or a tag name `[?]` | (all) `[?]`; changing Polygon Tag Type resets it to "(none)" `[?]` |
| Scope | enum: Surface, Fur, All | `[?]` |
| Apply to Sub-Group | bool | off `[?]` |
| Apply to Item Instances | bool | `[?]` (Q-22) |
| Scale block | UV repeat, Size, Bump Amplitude, Absorption, Displacement, Subsurface distances (each a checkbox) + Scale % | all off, 100% `[?]` |

An untagged group with Item "(all)" adds no mask: purely organisational.
Assign Material (M) on selected polygons creates a group with Polygon Tag
Type = Material, Polygon Tag = the new name, and a Material inside it.
`[D]`

### 6.3 Material `[D]`

Layer block + GL Display, then the surface channels. For the skeleton only
names and value types are needed; the decided direction is Principled-style
channels with friendly names:

Shading Model (enum; plan: Principled only), Diffuse Color, Diffuse Amount,
Diffuse Roughness, Specular Color, Specular Amount, Fresnel, Roughness,
Anisotropy, Reflection Color/Amount, Clearcoat Amount/Roughness,
Metallic, Sheen, Sheen Tint, Specular Tint, Flatness, Transparent
Color/Amount, Refractive Index, Refraction Roughness, Absorption Distance,
Dissolve, Subsurface Color/Amount/Distance, Luminous Color/Intensity,
Bump Amplitude, Displacement Distance, Smoothing (%), Smoothing Angle,
Double Sided.

(Smoothing already exists on Operandi's material; keep it there.)

### 6.4 Shader `[D]`

Layer block + GL Display; Compute Shading, Control Quality, Shading Rate,
Direct/Indirect Illum Multiplier, Control Visibility, Cast Shadows, Receive
Shadows, Visible to Camera / Indirect / Reflection / Refraction rays, Alpha
Type (Opacity, Constant, Shadow Catcher), Alpha Value, Control Light Linking
(+ light group, Include/Exclude), Control Fog (+ type, colour, distances,
density). Skeleton: the visibility block is enough.

### 6.5 Render Output `[D]`

Enable, Effect (output kind), Output Filename, Antialiasing, Image
Processing. Skeleton: Enable + Effect.

### 6.6 Texture layers `[D]`

Layer block + **Effect** + **Locator** (reference to its Texture Locator) +
**Projection Type** + **Projection Axis** (X/Y/Z).

Two-zone textures (procedurals, Vertex Map Texture): Value 1, Value 2,
Color 1 + Alpha 1, Color 2 + Alpha 2, Bias, Gain. Scalar effects use the
Values, colour effects use the Colors. `[D]`

- **Constant**: Color (or value).
- **Image Map**: Image (clip reference), filtering, Low/High Value, Alpha
  Channel (Use Alpha, Ignore, Alpha Only), Swizzling, Brightness, Contrast,
  Gamma, Clamp, Invert R/G/B.
- **Gradient**: Input Parameter, Value curve, Color curve.
- **Vertex Map Texture**: Vertex Map + two-zone block.

### 6.7 Texture Locator `[D]`

Projection Type (Solid, Planar, Cylindrical, Spherical, Cubic, Box, Front,
UV Map, Light Probe, Triplanar), Projection Axis, Front Culling, Blending
Hardness, Position, Rotation, Size, World Transforms, World Coordinates,
Horizontal/Vertical Repeat (Reset, Repeat, Mirror, Edge), UV Rotation,
Falloff. Skeleton: item + transform + projection type is enough.

## 7. Effects and blend modes (for menus)

**Texture effect menu, surface items** `[D]`, grouped:
- Basic Channels: Diffuse Color (default), Diffuse Amount, Diffuse
  Roughness, Specular Color/Amount/Fresnel, Reflection Color/Amount/Fresnel,
  Roughness, Clearcoat Amount, Transparent Color/Amount, Refraction
  Roughness, Subsurface Color/Amount, Luminous Color/Amount, Dissolve.
- Principled: Flatness, Metallic, Sheen, Sheen Tint, Specular Tint.
- Shader Control: Driver A, B, C, D; Group Mask; Layer Mask; Texture
  Offset.
- Special Effects: Anisotropy Direction, RGBA, Stencil.
- Surface Shading: Bump, Displacement, Normal, Object Normal, Vector
  Displacement.

Leave out for now: Fur, Hair, Cel, Surface Particle, Volumetrics.

**Shader effect menu** `[D]`: Full Shading (default), Diffuse Shading,
Specular Shading, Reflection Shading, Transparent Shading, Subsurface
Shading, Luminous Shading, Fog Shading.

**Render output effects**: Final Color, Alpha for now (more later).

**Blend modes** `[D]` (menu order as the guide lists them): Normal, Add,
Subtract, Difference, Normal Multiply, Divide, Multiply, Screen, Overlay,
Soft Light, Hard Light, Lighten, Darken, Color Dodge, Color Burn, Normal Map
Blend.

Rules `[D]`: a blend only combines layers with the **same effect**, and
only affects layers **below** it.

## 8. Evaluation rules

Not needed to render in ST-1, but the data model must support them. Items
marked `[M]` were measured; the rest remain `[?]`.

1. For a polygon, a layer applies if it is enabled and every enclosing
   group's mask matches the polygon (item AND tag; nested groups narrow). `[D]`
2. Per channel, applicable layers fold **bottom to top**:
   `result = lerp(below, blend(mode, below, layer), coverage)`. A layer at
   100 % Normal replaces the value below exactly; Multiply multiplies per
   channel. `[M]` Coverage = opacity × texture alpha × layer mask × group
   masks `[?]` (Q-17).
3. A Material writes all its channels at once. `[D]`
4. **Layer Mask** (surfaces): a layer whose effect is Layer Mask hides only
   the **one layer directly above it**; layers above that are unaffected.
   `[M]` (black mask under a red layer → red gone; same mask under a blue
   layer that sits on the red → red still shows.) A mask sitting directly
   under a Shader masks that shader, and the surface is not drawn at all. `[M]`
5. **Groups with textures but no material** start from whatever the tree
   below the group produced for that surface: a Multiply 0.5 texture alone
   in a masked group gave exactly half of the material below the group. `[M]`
6. **Several Group Mask layers in one group** combine with each other by
   **their own blend modes**, in stack order, like ordinary layers: two
   0.5 masks gave coverage 0.5 when the upper one was Normal and 0.25 when
   it was Multiply. (Replaces the v0.1 placeholder "multiply".) `[M]`
7. **Group blend mode and opacity.** A group behaves as one layer: its
   content folds on its own, starting from the **full channel set** produced
   below the group; then that whole result is blended onto what's below with
   the group's Blend Mode and Opacity, **every channel**, including channels
   no layer inside the group touched. `[M]` Measured (base luminous colour
   (0.8, 0.4, 0.2), luminous intensity 1):
   - Normal 50 % over { red Normal, Multiply 0.5 } → (0.65, 0.2, 0.1)
     (folded first, then mixed; not opacity per child)
   - Multiply 100 % over { Normal 0.5 } → (0.4, 0.2, 0.1); at 50 % → 0.75 × base
   - Add 100 % over { Normal 0.1 } → (1.8, 1.0, 0.6): the colour became
     base + 0.1 **and** the untouched luminous intensity also added to
     itself (1 + 1 = 2), doubling the pixel. Operandi must blend the whole
     channel set, not only the channels set by layers in the group.
   - Opacity 0 → base unchanged.
8. **Shaders: topmost applicable wins.** A Shader (camera visibility off)
   in an item-masked group had no effect while the group sat below the Base
   Shader, and hid the surface once the group sat above it. `[M]`
9. **Invert on a Material Group**: turning it on had no visible effect on a
   group holding one Normal luminous-colour layer. What it inverts is still
   open (Q-18); store the flag only. `[M]` + `[?]`
10. Polygons with an identical set of applicable layers share one compiled
    Cycles shader (see the earlier Cycles study's "surface contexts"). `[?]`

## 9. Suggested build slices

1. **ST-1 (this request)**: data model for the item types in §2 with the
   channels in §6, default scene (§3), tree panel with header, five columns,
   expand/collapse, select, rename, eye toggle, blend menu, opacity drag,
   effect menu, Add Layer (Folder, Group, Material, Constant, Image Map,
   Shader, Render Output), delete, drag reorder/parent, undo for all of it.
   Properties panel shows the channels. Save/load with the scene.
2. **ST-2**: Assign Material (M) creates tag + group + material; Select
   Polygons; masks by item/tag recorded (not yet rendered); Ctrl+G.
3. **ST-3**: Texture Locators, layer masks by drop, Filter, Materials view.
4. **ST-4**: first compile to Cycles (Base Material + material groups).

## 10. Panel conventions and texture defaults (from tutorial footage of earlier versions; provisional)

These come from watching recent-era tutorial videos, not 17.1 itself. Low
risk; build them as stated unless a later check says otherwise. `[?]`

- A Material Group row shows its mask kind after its name, greyed:
  "(Item)", "(Material)", "(Part)". A Part group with no tag yet is named
  "(none)".
- **M** opens the set-material dialog (name + colour). With polygons
  selected it tags those; with none selected, or in Item mode, it tags the
  whole mesh. Entering an existing name puts the polygons into that existing
  group instead of making a new one.
- Viewport right-click on a mesh → **Create Item Mask**: adds a group masked
  to that item with a new Material inside.
- Clicking a surface in the preview selects its material in the tree.
- Copy on one Material and Paste on another copies all values, with no link.
- An image dragged onto a material arrives as an image layer with Effect
  Diffuse Color.
- Add Layer with the Base Shader selected puts the new layer directly below
  it at the root.
- **New Gradient**: Effect Diffuse Color, Blend Normal, Input Parameter
  **Incidence**, one key, white; so it starts as flat white. Keys are added
  by middle-click on the curve; keys can be Stepped; the curve holds its end
  values outside the first and last key. Horizontal axis meaning: Incidence
  left = facing the camera; Slope left = facing world up (0°) to right =
  vertical (90°); Back Facing left = front; Thickness left = thin; Texture
  Value reads the layers below (dark left, light right).
- **Variation Texture**: Variation Source = Particle (default), Texture
  Particle, Mesh Part, Item. With Item, each item instance gets its own
  random value looked up on the gradient.
- Image layers map texture value 0 → Low Value and 1 → High Value;
  procedurals use Background / Foreground Value the same way.
