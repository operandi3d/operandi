# Preferences window: every option, build now or later (v2, 2026-11-06)

Behaviour-only spec. Replaces the 11-04 inventory (v1) as the planning list.
Source: the reference app's 17.1 user guide (Preferences chapter, 6 pages)
and its release notes 11.0 to 17.1. Panel order follows the reference
window's tree as Jason sees it. No screenshots in this folder.

## How to read the verdicts

- **NOW**: Operandi already has the feature this option controls. Build the
  option whenever the panel is picked up.
- **NOW-CHECK**: probably built already (earlier preference hand-offs on
  11-04 covered colours, undo/auto-save, units/grid, selection, navigation,
  handles/snapping, overlays, mesh/tool defaults; at least Colors, Undo,
  Auto-Save and 3D Information Overlays are in main). The build thread
  should tick off what exists before writing code.
- **LATER (feature)**: the option is only meaningful once Operandi has that
  feature. Build it together with the feature, not before.
- **SKIP**: tied to something Operandi will not have (a third-party
  renderer, network rendering, telemetry, another app's file format nobody
  asked for, a legacy switch). Leave out.

"What Operandi has" comes from project notes and specs, not from reading
the code: modelling tools and toolbox, component and item modes, mid-tool
undo, falloff, symmetry, action centres, vertex + grid snapping,
subdivision (both kinds) with edge weights, work plane and grid, units,
materials via the layered shader tree, Cycles rendering with a progressive
Preview window and in-viewport render preview, cameras and lights,
backdrop items, item parenting, drop actions, selection sets, keybinding
editor, colour themes, 3D information overlays, file formats OBJ, STL,
glTF, GEO, SVG and LXO. Not yet: animation, UV editing, painting,
sculpting, image layers, procedural mesh ops, Booleans, MeshFusion,
instances, a preset browser, colour management, render outputs/passes,
baking, scene references.

## How the window behaves (all panels)

- PW-1. One window: a tree of sections and panels on the left, the picked
  panel's controls on the right. Sections: Data, Defaults, Display, File
  I/O, Input, Rendering (alphabetical in 17.1).
- PW-2. Changes apply at once; there is no OK or Apply. Values persist
  between launches in the user's settings.
- PW-3. A "reset preferences" command exists outside the window (System
  menu, also offered in Safe Mode) that restores factory values.
  Operandi suggestion: also give each panel a "Reset panel" button; it is
  cheap and saves users hunting.
- PW-4. Several panels are not settings but small editors (lists you add
  rows to): Item Tag Definitions, Path Aliases, Preset Markup Definitions,
  Drop Mappings, Game Input Mappings, Host List, PBR Defaults.
- PW-5. Where a preference sets the default for new scenes (frame rate,
  scene range, up axis, colour config), each scene can still override it
  in its own Scene properties, and a Default Scene (see Application)
  overrides these preferences.

---

## Data

| Option | What it does | Default | Verdict |
|---|---|---|---|
| Item Tag Definitions (editor) | Names, descriptions, tooltips and line counts for four-letter item tags shown on an item's Tags tab; tags can be hidden | — | LATER (item Tags tab; it is in the camera spec's shared item tabs) |
| Path Aliases (editor) | Alias name to folder; linked files under that folder are stored by alias so scenes move between machines. Right-click: rename, change path, duplicate, delete | — | LATER (linked files: image maps) |
| Preset Markup Definitions (editor) | Metadata fields for presets in the preset browser | — | LATER (preset browser) |
| Referencing: Allow Channel / Name / Structural Overrides, Allow Deletions, Allow 3D Selections | Default override rules for referenced scenes. Each: Always Allowed, Never Allowed, If Allowed by Item, Unless Disallowed by Item | — | SKIP (no scene references planned) |

## Defaults

### Animation

All LATER (animation). Listed so nothing is lost:
Channel Controls (keyframe dots beside fields), Time System (Frames,
Seconds, SMPTE, Film Code), Frames per Second (Film 24, PAL 25, NTSC 29.97,
Custom; default 30 since 17.0) and Custom Rate, Play Real Time, End
Behavior (Hold, Repeat (default), Wrap), Preserve Key Timing, Scene Range,
Auto Key (All, Animated, None), Auto Update Existing Keys, Time Snap, Slope
Type, Initial Weight, Channel Behavior, Channel Interpolation, Link Keys.
Cache Deformations was removed in 17.0.

One exception: **Rotation Order** (default order for new items' rotation,
XYZ etc.). NOW only if Operandi items already carry a rotation-order
property; otherwise LATER (item transform properties).

### Application

| Option | What it does | Default | Verdict |
|---|---|---|---|
| Initial Selection Type | Selection mode when the app opens | — | NOW |
| Max Undo Levels | Steps kept in undo history | 100 | NOW-CHECK |
| Group Selection Cmds | Consecutive selection clicks become one undo step | on | NOW-CHECK |
| Max Sub-Commands Recorded | Cap on clicks per grouped step | — | NOW-CHECK |
| Default Material | Material tag for new polygons; prefill in Set Material | — | NOW |
| Default Part | Prefill name for Set Part | — | NOW (Set Part exists) |
| Default Texture Map | Name for new and auto-created UV maps | — | NOW (UV maps exist, editing doesn't) |
| Default Vertex Normal Map | Name for new vertex normal maps | "Vertex Normal" | LATER (vertex maps) |
| Default Multiresolution Layer | Name for new sculpt layers | — | LATER (sculpt) |
| Item Index Style | How a number is added when a name is taken (Item (2), Item(2), Item 2, Item_2, Item2). Applies to items, shader tree layers and vertex maps | — | NOW |
| Use OS Color Picker | System colour dialog instead of the built-in picker | off | NOW (Operandi has its own picker; macOS panel is easy) |
| Max Recents in Categorized Lists | Length of the "Recent" list at the top of Add menus (e.g. shader tree Add Layer) | — | NOW if Add menus have a Recent list, else LATER |
| Max Recents in Searches | How many past search terms list filters remember | — | LATER (list search fields) |
| Reset "In the Future..." Messages | Brings back every dialog the user chose to hide | — | NOW if Operandi has any "don't ask again" dialogs |
| Automatically Check for Updates | Never, Every Launch, Daily, Weekly; only alerts, never installs | — | LATER (public release + update feed) |
| Automatically Submit Usage Statistics | Never / Every Launch / Daily / Weekly | — | SKIP (open-source project; no telemetry) |
| Default Scene | A scene file loaded for every new scene and at launch; its settings beat scene-related preferences | empty | NOW (LXO load and New Scene exist) |
| Preset Browser: Content Location, Dir Cache Updates (Only When in Use / Always Keep Updated, default Always since 12.0), Scan For Changes Using (Directory time only (default) / File and directory times), Lock Preset Thumbnails | Where presets live and how the browser cache refreshes | — | LATER (preset browser) |
| Foundry Link: Address, Port | Bridge to a game engine | — | SKIP |

### Auto-Save

| Option | What it does | Default | Verdict |
|---|---|---|---|
| Enable | Periodically writes a backup copy of the scene (and changed images) | — | NOW-CHECK |
| Time Interval | Minutes between backups | — | NOW-CHECK |
| Backup Directory | Where backups go; separate from the user's file | — | NOW-CHECK |
| Number of Revisions | Backups kept per scene (since 11.1) | — | NOW-CHECK |
| Progressive Render Auto-Save: Enable Image Auto-Save, Write Image Sequence, Save Layered Image, Time Interval | Periodically saves the Preview render to the backup folder as EXR | — | LATER (EXR save from Preview) |

Behaviour: before each backup a 10-second countdown with Cancel shows in
the 3D view; Cancel skips that one only. Backups older than 31 days are
deleted at quit. Saving can briefly slow the app.

### Experimental

| Multi-Threaded Drawing | Viewport geometry prep on several threads instead of one; off by default | off | SKIP as a preference (an internal choice; covered by architecture item #9) |

### Images & Painting

All LATER (image layers / painting): Enable Image Antialiasing by Default
(on), Default New Image Resolution, Always Display Images as Icons, UDIM is
(number before / after), Threaded Painting (on; SKIP, internal), Auto
Create Textures, Default Foreground Color (White, Black; White).

### Mesh Data Evaluation

| Option | What it does | Verdict |
|---|---|---|
| Defer Smoothing During Interaction | Skip vertex-normal smoothing while dragging; recompute on mouse-up | NOW (cheap speed win on big meshes) |
| Compute Smoothing During Animation Playback | Same during playback | LATER (animation) |

### Mesh Items

| Option | What it does | Verdict |
|---|---|---|
| Auto Create Item Masks | New mesh items get their own shader tree mask | NOW (shader tree has masks) |
| Auto Create Preset Item Masks | Same for items added from presets | LATER (presets) |
| Auto Create Item Transforms | New items get position/rotation/scale channels at once | NOW-CHECK (likely moot: Operandi items always have transforms) |
| Auto Hide Child Items | Hiding a parent hides its children | NOW (parenting exists) |
| Auto Create UV Map | New mesh items get an empty UV map (added 16.0) | NOW |
| Create Instances as Bounding Boxes | Instances draw as boxes | LATER (instances) |
| Save Vertex Maps Selection to Scene | Selected vertex maps stay selected after reopen | LATER (vertex map list) |
| New Patch Display Level | Starting subdivision level for new meshes (Modo SubD) | NOW |
| New Catmull-Clark Display Level | Same for Catmull-Clark | NOW |
| New Curve Display Level (Coarse, Medium, Fine), New Curve Display Angle | Curve smoothness for new curve items | LATER (curve items) |
| New Spline Patch Display Level | Spline patch level | LATER (spline patch) |
| Default Primitive Polygon | Default polygon-type option in primitive tools (Sphere, Torus...) | NOW |
| Symmetric Primitive Creation | With symmetry on, primitives are also created mirrored | NOW |
| 3D Editing: Flatness Limit | Threshold for "non-planar" polygons (used by Statistics selection) | NOW if a non-planar selection exists, else LATER (Statistics) |
| Copy and Paste: Deselect Elements after Copying / Select Pasted Elements / Deselect Elements before Pasting | Selection behaviour around copy/paste; default leaves the old selection and pasted geometry unselected | NOW |
| Slip UV: Startup State (Off (default), Last Selected) | Whether tools start with Slip UV off | LATER (UV editing) |

### Procedural Modeling

| Option | Verdict |
|---|---|
| Draw Tool Handles (mesh-op handles) | LATER (procedural mesh ops) |
| Ghosted Mesh Drawing: Drawing Style (Shade, Wireframe, Hide), Color, Opacity | LATER (procedural ghost meshes) |
| MeshFusion Wire Opacity | LATER (MeshFusion, which is #2 on Jason's future list) |

### Scene Save/Export

| Option | What it does | Verdict |
|---|---|---|
| Author, Copyright | Written into saved/exported files that support it | NOW (LXO; glTF has an asset copyright field) |
| Absolute Path | Linked files stored with absolute instead of relative paths | LATER (linked image files) |
| Merge Reference Items | Embed referenced scenes on save | SKIP |
| Text Encoding (UTF-8, system default) | Encoding of saved text | SKIP. Sane choice: always UTF-8 |

### Switcher Bar

Autohide; New Window Defaults: Window Edge (top, bottom, left, right),
Initially Collapsed, Show When Collapsed, Show Only Favorites; New Palette
Defaults: same four. Controls the layout-switcher strip.
Verdict: LATER (only if Operandi grows a layout switcher with favourites
and floating palettes). Autohide alone is NOW if Operandi's workspace tabs
want it.

### Tool Settings

| Option | What it does | Verdict |
|---|---|---|
| Default Action Center (component modes / item mode) | Action centre at startup, or Last Selected | NOW |
| Default Action Axis (component / item) | Action axis at startup | NOW |
| Default Brush (component / item) | Brush tip at startup | LATER (sculpt/paint) |
| Default Constraint (component / item) | Constraint at startup (e.g. to background) | LATER (constraints) |
| Auto Activate | Makes the per-tool Auto Activate a global switch | NOW if tools have Auto Activate |
| Curve Tool Auto Activation | Selecting a curve starts its curve tool | LATER (curve items) |
| Select Through | Makes the per-tool Select Through a global switch | NOW if tools have Select Through |

### UI

| Option | What it does | Default | Verdict |
|---|---|---|---|
| Tidy Layouts | Removes doubled borders between nested panels | on | SKIP (Operandi's layout has no doubled borders) |
| Popup Transparency | Transparency of popups, popovers, pie menus | — | NOW |
| Popover Dismissal Behavior | Default / Force Click-Off / Force Roll-Off for simple popovers | Default | NOW |
| Base Form Proficiency Level | Basic / Standard / Advanced: how many controls tool forms show; Less/More buttons at the bottom | Standard | LATER (tool forms tagged by level) |
| Default Form Icon Size | Small / Normal / Large | Normal | NOW |
| UI Images: Sort By, Maximum Images, Image Cache Size, Thumbnail Size, Clear Images From, Clear All UI Images | Image/brush/clip browsers | — | LATER (painting, image browsers) |
| Preset Browser: Show Nice Names, Show Extensions, Show Info Badge Only | Preset browser display | — | LATER (preset browser) |
| Tool Properties: Modo Layout (Popover, Inline, Split) | Where tool properties show in the main layout | — | NOW if Operandi wants more than one placement; else SKIP |

## Display

### 3D Information Overlays

Built in main (lower right; Current Selection, GL Polygon Count and Grid
Size on by default). Remaining toggles:

| Overlay | Verdict |
|---|---|
| Current Selection, OpenGL Polygon Count, Grid Size | NOW-CHECK (built) |
| Polygon Type (Face, Subdiv, Catmull-Clark, mixed) | NOW |
| Current Tool (with falloff, action centre, snap) | NOW |
| Snapping State | NOW |
| Morph Map | LATER (morphs) |
| Reference Item | SKIP (references) |
| Selected Channel Count | LATER (channels / animation) |
| Deformers Enable State | LATER (deformers) |
| Active Passes and Actions | LATER (passes / actions) |
| UV Coverage | LATER (UV editor) |

### Colors

Built in main (themes, schemes). Not yet built, per colour-themes notes:
per-view schemes, rollover colours by kind (Generic, Face, Subdivision,
Catmull-Clark), falloff range (Cool, Warm), topo colours, boundary
highlights. Of those: rollovers and falloff range are NOW; UV Boundary,
Geometry Boundary NOW if those view options exist; Topo LATER (topology
mode); PSUB Mask LATER (sculpt); SelOp LATER (procedural); Discontinuous
Pair LATER (UV editor).

Reference oddity, do not copy: edited colours don't change any view until
saved as a scheme and picked per view. Operandi already applies them live,
which is the sane choice.

### OpenGL

| Option | What it does | Default | Verdict |
|---|---|---|---|
| Texture Resolution | Largest texture drawn before downsampling (powers of 2) | — | LATER (image textures in viewport) |
| Environment Resolution | Largest environment image drawn, 64×32 to 16k×8k | 4096×2048 | LATER (environment images) |
| Flatness of Perspective | Strength of perspective in 3D views; higher = flatter | 60% (since 12.0) | NOW |
| Max GL Polygon Count | Above this, an operation that would exceed it offers to cancel | — | NOW |
| Bounding Box Threshold | None / All Surface Items / Only Inactive Surface Items: over the cap, items draw as boxes while navigating. Items can opt out | — | NOW |
| Bounding Box Drawing in Animation | Same during playback | — | LATER (animation) |
| Enable Displacement in GL | Displacement drawn as real geometry, not bump | — | LATER (displacement in viewport; bump/normal/disp branch is render-side) |
| Grid Visibility | Default grid visibility in new 3D views | on | NOW |
| OpenGL Point Size | Drawn vertex size in pixels; also the vertex click area | 3 | NOW |
| Selected Point Size | Multiplier for selected vertices | — | NOW |
| Viewport Fit Factor | Margin for Fit All / Fit Selected (1 = touches the shorter edge) | 1.5 | NOW |
| Topology Vertex Backface Culling | Topo mode hides back-face vertices | — | LATER (topology mode) |
| Weight Value Precision | Decimals for shown weight values | — | LATER (weight maps) |
| Transparency: Selection | X-ray opacity of selection seen through surfaces | — | NOW |
| Transparency: Handles | Opacity of handles behind surfaces | — | NOW |
| Transparency: Topology Polygons | Front-face transparency in Topo mode | — | LATER (topology mode) |
| Selection Rollovers | None / Manual (current mode) / Closest (nearest element of any kind) | — | NOW |
| Mouse Regions Trigger Selections | Right-click an element to switch to its component mode | — | NOW |
| VBO Mode, AVP Allow Material Texture storage, max requested layers | Graphics-driver workarounds | — | SKIP |
| Only Draw Selected Procedural Outlines | Procedural items as outlines | — | LATER (procedural items) |
| Sync Display Refresh | V-sync for viewports | — | NOW |
| Sync Application Response | Ties UI to viewport redraw | — | SKIP |
| Performs Color Correction on OpenGL Objects | Viewports follow colour management | — | LATER (colour management) |
| Backdrop Item: Include Backdrops in Viewport Fitting, Display Backdrops in Uniform Size, Pixel Size | Fit includes backdrops; size from item scale; real size of one pixel on import (10 mm → 512 px = 5.12 m) | — | NOW (backdrop items exist) |
| Item Drawing: Default Size for New Scenes / Current Scene | Draw size of cameras, lights, locators | — | NOW (cameras/lights exist) |
| Item Drawing: array drawing toggles (16.1) | Draw array items | — | LATER (arrays) |
| Schematic Grid Snapping | Node snap step in the schematic | 2.0 | LATER (node editor, if node materials happen) |

### RayGL

All SKIP: Resolution (Full, Half, Quarter), Updates (Delayed, Lazy,
Synchronous), Quality (Draft, Final Render, Extended Refinement Passes),
effect toggles, Draft settings (displacement rate, quality %, AA samples,
max GI bounces 50%, max MC/IC rays). This is the reference's own ray-traced
viewport. Operandi's equivalent is the Cycles render preview; if it needs
settings (resolution divider, update on mouse-up vs live), give them
Operandi's own names under Rendering > Preview, not here.

### Tool Handles

| Option | What it does | Verdict |
|---|---|---|
| Large / Small Points | Drawn size of handle points (px) | NOW |
| Large / Small Points Hit Size | Click radius of handle points | NOW |
| Handle Line Width, Handle Line Hit Width | Drawn and clickable width of handle lines | NOW |
| Handle Scale: Standard | Overall on-screen handle size; also the - and = keys | NOW |
| Handle Scale: Small (% of Standard), Plane Handle Ratio | Secondary and two-axis handle sizes | NOW |
| Draw Style: Selected Tool Handles (Invisible, Basic, Advanced) | Handle detail for the active tool; Advanced adds extras (e.g. camera focus) | NOW |
| Draw Style: Unselected Tool Handles | Detail for secondary handles (falloff etc.) | NOW |
| Auto-Haul Display (Off, Basic, Advanced, On) | On-screen slider helpers for some tools | LATER (auto-haul helpers) |
| Handle Snapping: Inner Range, Outer Range (px) | Snap distance and pre-highlight distance | NOW-CHECK (snapping spec: 24 / 40 px) |
| Rotation Snap Angle | Step for Ctrl-constrained rotation | NOW |

## File I/O

Operandi reads/writes OBJ, STL, glTF, GEO, SVG, LXO today.

### Wavefront I/O — NOW

Import: Import as Static Mesh (LATER, needs static meshes; default on),
Import Groups as Separate Meshes (off), Suppress Import Options Dialog
(off; NOW only if an import dialog exists), Import Units (meters,
centimeters, millimeters, inches; meters).
Export: Save Meshes as Groups (g instead of o; off), Save Material File
(.mtl; on), Save deformed geometry at current scene time (LATER, deformers),
Export Units (meters).

### STL I/O — NOW

Interpret Export units as, Save as ASCII (off = binary), Interpret Import
units as. Jason wants STL export (11-06); build these with it.

### glTF v2 I/O — NOW (geometry parts) / LATER (animation parts)

NOW: Export Type (Export All, Export Selected (no hierarchy), Export
Selected with hierarchy; All), Save Geometry (on), Save Materials (on),
Save Tangent Basis (off), Indices data type (Unsigned Short, Unsigned
Integer; Integer).
LATER (animation/skinning): Save Animation, Save Weight Maps, Save Skinned
Meshes in World Space, Animation Sample Rate, Export Actions.

### Image I/O

| Option | Verdict |
|---|---|
| Default Image Format (for Save As of rendered images) | NOW if renders can be saved |
| PNG Compression Level (1–9) | NOW if PNG save exists |
| JPEG Quality, Sub-Sampling (4:1:1 default, 4:2:2, 4:4:4) | NOW if JPEG save exists |
| OpenEXR Export: Compression, DWA Level (45), Tile Size, Generate Mip Maps | LATER (EXR save; noted missing) |
| Image Cache: Limit Image Load Size, Maximum size, Image Tile Cache Size | LATER (image textures) |
| OpenEXR Loader: Limit Tile Memory, EXR Tile Memory Limit, Cache Tiles to Disk | LATER (EXR textures) |

### PBR Defaults

The PBR loader's name rules: which file-name words map to which texture
effect (e.g. "basecolor" → Base Color, several words split by ";"), plus
variables (Create Folder, Use Folder Name, one Texture Locator, Select
Created Items, Match Existing Groups, Set Type on Rename, Create Material
type). LATER (image textures in the shader tree + a "Load PBR set"
command). Worth building then: it is a big time-saver.

### Later formats (build with the format)

- **USD I/O**: Convert Distance Units (on), Convert Light Intensity (on),
  Light Intensity Scale Factor. LATER ("Export to USD" idea).
- **FBX I/O**: preset (none, Maya, Unity/Unreal static/animated), export
  file version (Latest), Export Type, Save as Text, Save Only Animation,
  Save Geometry/Materials/Cameras/Lights/Animation, Surface Refining (None,
  Save Mesh Smoothness, Triangulate), Polygon Parts, Selection Sets,
  Smoothing Groups, Morph Maps, Tangent Basis, Units, Scale Multiplier,
  RGBA as Diffuse, PBR XML, Embed Media, Sample Animation + rate, Export
  Actions; import options (merge with existing, geometry, normals,
  smoothness, blend shapes, parts, selection sets, materials,
  transparency handling, cameras, lights, animation, global scale, units).
  LATER (FBX support). Legacy-plugin switch: SKIP.
- **Alembic I/O**: time range, export type, which item kinds, UVs,
  normals, material/part tags, name cleanup, scale. LATER (animation).
- **Movie I/O**: Default Video Format. LATER (animation render).

### Skip

Collada I/O, LightWave I/O, HPGL Plotter I/O, Rhino I/O (ask settings on
import, import lights). Nobody has asked for these formats.

## Input

### 3D Mouse

All LATER (3D mouse hardware support): Navigation Style (Object Orbit,
Camera Walkthrough), Orthographic View Style (World Space, Screen Space,
Screen Space with Y/Z swapped), Use Modifiers to Toggle Styles, Speed, Swap
Y and Z, Focus on Viewport Under Mouse, Show Rotation Center Marker, Lock
Horizon, Dominant Axis Only, per-axis lock/invert, thresholds, speeds,
non-linear exponents, Global Exponent Multiplier.

### Drop Mappings

Editor: every modifier combo (plus unmodified) with an ordered list of
drop actions; per-viewport overrides; a source filter; actions can be put
in "never map". Meant for advanced users. LATER (after the drop-action
popover and viewport drop gesture land).

### Game Input Mappings

Key mapping editor for game-style (WASD) navigation, QWERTY/AZERTY switch.
LATER (game navigation mode), likely SKIP.

### Remapping

| Option | What it does | Default | Verdict |
|---|---|---|---|
| Mouse Input Preset | Navigation mapping that mimics other apps | native | NOW-CHECK (Operandi has native/Blender/Maya) |
| Input Device | Mouse (hauls continue past screen edge), Tablet (stop at edge), Hybrid (mouse for hauls, tablet for navigation; new in 17.1) | Mouse | NOW |
| Allow Modifier Chording | Modifier keys can take part in mouse chords | — | NOW |
| Numeric Enter as Tab | Keypad Enter moves to the next field | — | NOW |
| Open Dialogs Under Mouse | Dialogs open at the pointer instead of last place | — | NOW |
| Tooltip Delay | Hover time before tooltips | 0.8 s | NOW |
| Mouse Wheel Acceleration | Wheel zoom speed (%) | — | NOW |
| Wheel Zoom at Mouse Cursor | Zoom centres on pointer, not view centre | — | NOW |
| Invert Mouse Wheel Zoom | Reverse wheel zoom | — | NOW |
| Reverse Pan Widget Direction | Reverse on-screen pan widget | — | NOW if the pan button exists |
| Zoom Drag Axis | Left→Right, Right→Left, Bottom→Top, Top→Bottom | — | NOW |
| Trackball Rotation | Free orbit instead of two-axis orbit; views can override | off | NOW (Operandi has Off only) |
| Override Viewport-Level Trackball Option | Forces every view to follow the preference | — | NOW (with trackball) |
| Trackball Speed Multiplier (Mouse / Tablet) | Trackball orbit speed | — | NOW |
| Invert Drag Scrolling, Invert Mouse Wheel Scrolling | Scrolling direction in lists, trees, forms | — | NOW |

Reference extra, do not copy: first orbit after install pops a 17-second
"try trackball?" prompt.

### Selection

| Option | What it does | Default | Verdict |
|---|---|---|---|
| Polygon Selection (paint/lasso) | Frontface & Wireframe / Frontface & Backface / Frontface only / Always Raycast / No Raycast | Frontface & Wireframe | NOW |
| Double Sided Selection | Polygons with a double-sided material can be paint-selected from behind | — | NOW if materials have Double Sided |
| Selection Hit Size | Click radius (px) around a pre-highlighted element | — | NOW |
| Point Selection Priority | Mixed-element tools favour points over edges near corners | — | NOW (Tweak) |
| Lasso: Ignore Shading Style | Lasso picks the same in wireframe and shaded | — | NOW |
| Lasso: Allow Partial Polygons Selection | Lasso picks polygons it only touches | — | NOW |

### Units

| Option | What it does | Default | Verdict |
|---|---|---|---|
| Unit System | SI, Metric, English, Game Units, Unitless | — | NOW-CHECK |
| Default Unit | Unit assumed when none is typed (depends on system) | — | NOW-CHECK |
| Inch Scale Cutoff | English: above this, show feet + inches (36: 27" stays, 42" → 3' 6") | 36 | NOW |
| Meters per Game Unit | Size of one game unit | — | NOW |
| Dynamic Units: Force, Acceleration, Speed, Mass (system + default unit each) | Dynamics units | — | SKIP until dynamics |
| Light Unit System | Radiometric / Photometric light intensity | — | LATER (light properties tier 1) |
| Color Unit System | Floating Point, Percentage, Integer (0–255), Hexadecimal | — | NOW |
| Factor or Range | Show 0–1 factors as % or float | — | NOW |
| Coordinate Rounding | None, Normal, Fine, Fixed, Forced Fixed: how pointer positions round through the work plane | — | NOW |
| Fixed Increment | Step for the two Fixed modes; Fixed also floors the grid size | — | NOW |
| Up Axis | X, Y or Z is up; scenes can override | Y | NOW but Tier 2: touches grid, navigation, primitives, import/export |

### Work Plane

| Option | What it does | Verdict |
|---|---|---|
| Grid Type | Fixed or Dynamic (follows zoom) | NOW-CHECK |
| Size | Square size when Fixed; falls back to dynamic when lines crowd | NOW-CHECK |
| Preferred Plane | Plane the work plane leans toward | NOW |
| Preferred Plane Bias | How strongly (0% = none) | NOW |
| Lock Preferred Plane | Work plane stays on the preferred plane whatever the view | NOW |
| Locked Plane Offset | Offset of the locked plane from the origin | NOW |
| Plane and Grid Increments | How grid density steps with zoom | NOW |

## Rendering

### Color Management — LATER (colour management)

Default OCIO Color Config, Default 8-bit / 16-bit / Float Colorspace,
Default Numeric Colorspace (sRGB), Default View Colorspace, Affect Color
Swatches and Picker, Affect System Color Dialog, Apply Color Correction
Profile to Legacy LXO files (off). Cycles uses OCIO, so this panel comes
for free-ish once Operandi picks a colour pipeline.

### Final Rendering

| Option | What it does | Verdict |
|---|---|---|
| Automatic Render Threads, Render Threads | Thread count for final renders | NOW (Cycles threads) |
| Geometry Cache Size | RAM cap for render geometry | SKIP |
| Network rendering (Use Network Render Nodes, Network Nodes Only, Whole Frames, Group, Job Size, Send Assets, Shared Directory, Bonjour, Host List) | Render farm | SKIP |
| Default Output Gamma | Gamma of new render outputs | LATER (render outputs) |
| Color Picker Stops, Affect Color Control Swatches | Exposure control in the colour picker for HDR colours | LATER (HDR picker) |
| Default Field Rendering | Interlaced frames | SKIP |
| Bake UV Border Size | Border expansion when baking (32 px since 17.0) | LATER (baking) |
| Max Render Slots, Recent Frames Directory, Frame Cache Size | Render history kept on disk | LATER (render history window) |
| Write Buckets Directory | Bucket dump folder | SKIP |
| Default Output Pattern | Naming pattern for rendered frames | LATER (animation render) |
| Block System Sleep While Rendering | Stops the Mac sleeping mid-render | NOW (small; note the reference's own macOS version was broken until 17.1) |
| Auto Add | Render passes auto-add default | SKIP |
| VDB Voxel Cache: Cache Mode, Cache Size | VDB playback | SKIP |

### Preview

| Option | Verdict |
|---|---|
| Use Secondary Irradiance Cache | SKIP (Cycles has no irradiance cache) |
| Extended Refinement Passes | NOW as Operandi's own "Preview max samples" (Cycles equivalent) |
| Bucket Order (Semi-Random, Scanline) | SKIP (progressive Cycles) |
| Start Paused | NOW (Preview window exists) |
| Preview Thumbnail | LATER (material presets) |
| Use Subdivs Render Level, Use Catmull-Clark Render Level, Use Cage Mode | NOW if mesh items have a separate render level; else LATER |
| Render Proxy Items (Initial Value) | SKIP |

### Skip

Host List (network render hosts), Octane (third-party renderer), PView
(old ProRender viewer; only option is Start Paused).

---

## Build tiers (proposal)

**Tier 1: small, high value, feature already exists.** Each is mostly
"store a value, read it where the behaviour already lives".
1. Remapping: wheel zoom at cursor, invert zoom, wheel acceleration, zoom
   drag axis, trackball + override + speed, tooltip delay, dialogs under
   mouse, keypad Enter as Tab, list scroll inversion.
2. Selection: polygon selection mode, hit size, point priority, lasso
   options; OpenGL: selection rollovers, mouse regions, point size,
   selected point size, x-ray transparency for selection and handles.
3. Viewport basics: grid visibility, flatness of perspective, fit factor,
   item drawing size, backdrop fitting/pixel size.
4. Tool Handles panel (all except auto-haul) and Tool Settings default
   action centre/axis per mode.
5. Application: default material/part/texture map names, item index
   style, default scene, OS colour picker.
6. Check-and-finish: undo, auto-save, units, work plane, overlays, colours
   rollovers/falloff range (NOW-CHECK rows).

**Tier 2: needs a bit more work or touches several systems.**
Mesh Items group (auto UV, item masks, display levels, default primitive
polygon, symmetric primitives, copy/paste trio, auto-hide children);
Max GL polygon count + bounding-box threshold; Defer Smoothing During
Interaction; coordinate rounding + fixed increment; colour unit system and
factor display; Up Axis; Input Device (Mouse/Tablet/Hybrid); File I/O
panels for OBJ, STL, glTF and image save; Author/Copyright; render
threads, Block System Sleep, Preview start paused and max samples;
remaining overlays (polygon type, current tool, snapping state); UI popup
transparency, popover dismissal, icon size.

**Deferred, grouped by what they wait on.**
Animation (Animation panel, playback options, Alembic, Movie, glTF/FBX
animation parts) · UV editing (UV coverage, Slip UV, discontinuous
colour) · painting/sculpt (Images & Painting, brushes, multires, PSUB
mask, UI image browsers) · image textures (Image I/O caches, EXR, PBR
Defaults, path aliases, absolute paths, texture/environment resolution) ·
procedural mesh ops and MeshFusion (Procedural Modeling panel) · preset
browser (preset settings, markup, thumbnails) · item Tags tab (Item Tag
Definitions) · colour management (whole panel) · render outputs, passes,
baking, render history · topology mode · weight maps · light units (with
light properties) · 3D mouse · new formats (USD, FBX).

**Skip.** Octane, PView, RayGL, Host List and all network rendering,
usage statistics, Foundry Link, Collada, LightWave, HPGL, Rhino,
referencing, text encoding, driver workarounds (VBO, AVP, sync app
response), Tidy Layouts, field rendering, VDB cache, write buckets,
irradiance cache, bucket order, render proxies, experimental
multi-threaded drawing.

## Open questions

- PREF-Q1 (carried): factory values for rows marked "—". The guide gives
  only some; the rest need a read-only preference dump on the bridge.
  Not needed to start Tier 1: Operandi can pick sane values.
- PREF-Q2 (carried): is there a per-panel reset in the reference? Guide
  only shows the global Reset Preferences. Operandi can add one anyway.
- PREF-Q5 (new): Octane and Rhino I/O panels aren't in the 17.1 guide.
  Both are SKIP, so no research planned.
