# Common item tabs: behaviour spec v1 (2026-11-06)

The Display, Assembly, User Channels and Tags tabs of the Properties panel.
They are **the same on every item kind** (mesh, camera, light, locator,
backdrop...) `[D]`; build them once and show them for any selected item.
Written for cameras first (`BEHAVIOR-CAMERA-PROPERTIES.md`); the lights spec
reuses this file. Kind-specific extras (e.g. the camera's Show Camera
Overlay) are marked and belong to that kind's spec.

Tags as in the camera spec: `[D]` user guide, `[O]` scripted readback,
`[S]` seen on Jason's camera panel, `[pick]` our choice, `[?]` unknown.
`Op` = what Operandi has (camera/light work only; this thread did not read
Operandi code). Tier as in README.

Option lists confirmed by readback `[O]`: Visible = Default, Yes, No,
No (Hide Children); Pivot, Center, Select and Lock = Default, Yes, No.
An item also has a **Render** setting (Default, Yes, No) that is not on the
camera panel; lights show it (CL table).

## Display tab

**What it's for, in plain English:** how the item is *drawn in the viewport*, never how it renders. Hide it, show its centre marker, make its icon bigger, stick a name label next to it, give it a custom wire colour. The "Add ..." buttons bolt extra on-screen gadgets onto the item (dimension rulers, sticky notes, gauges, a trail showing its animation path); each one adds its own extra tab when switched on.


| ID | Label | Type | Default | Behaviour | Op | Tier |
|---|---|---|---|---|---|---|
| CP-D01 | Visible | dropdown: Default, Yes, No, No (Hide Children) | Default `[S][D]` | Default = follow the viewport's own show toggle for this kind of item (for cameras/lights, Show Cameras/Lights, CL-11); Yes = always drawn; No = never drawn; No (Hide Children) = this item and its children hidden `[D]`. | No | 1 |
| CP-D02 | Pivot | dropdown: Default, Yes, No | Default `[S]` | Shows the item's pivot marker `[D]`. | No | 2 |
| CP-D03 | Center | dropdown: Default, Yes, No | Default `[S]` | Shows the item's centre marker `[D]`. | No | 2 |
| CP-D04 | Size | float multiplier | 0.0 cameras, 1.0 lights `[O]`; others `[?]` | Scales the item's viewport glyph. 0 on a camera = constant screen size (CL-13) `[O]`. Not on mesh items (they have no glyph) `[pick]`. | Have (cams/lights) | 0 |
| CP-D05 | Add Doc Bounds | dropdown "...", greyed `[S]` | – | Undocumented `[?]`. Skip. | – | never |
| CP-D06 | Label | text | empty | Text drawn next to the item in the viewport. Typing one reveals **Show Label** (bool, on) and **Label Offset X/Y** (screen offset) `[D]`. | No | 1 |
| CP-D07 | Add Draw Options | toggle button (becomes Remove Draw Options) | – | Adds rows: **Enable** (bool), **Wireframe** (Default, User, Background Light, Background Dark), **Wireframe Color** (RGB, used when User), **Fill** (same four), **Fill Color** (RGB) `[D]`. Pick for Operandi: Enable + Wireframe + Wireframe Color only (cameras have no fill). | No | 2 |
| CP-D08 | Add Dimensions | toggle button | – | Bounding-box dimension readout + its own tab (Visibility Off/On/Selected, Opacity, per-axis show, min/max warn sizes, link/chain children) `[D]`. Useful on meshes; hidden on cameras/lights. | No | 3 |
| CP-D09 | Add Notes | toggle button | – | Up to nine lines of text shown when selected, own tab (Title + lines) `[D]`. | No | 3 |
| CP-D10 | Add Meter | toggle button | – | On-screen dial gauge driven by a channel, own tab `[D]`. | No | never |
| CP-D11 | Add Level Bars | toggle button | – | On-screen bar graph of several channels, own tab `[D]`. | No | never |
| CP-D12 | Show Camera Overlay | toggle button, **camera only** | off | Adds a **Camera Overlay** tab (CP-O below). Overlay draws only in a viewport looking through this camera `[D]`. | No | 2 |
| CP-D13 | Show Stereo Volume | toggle button, **camera only** | off | Draws the stereo depth budget (front/back parallax planes) in the viewport; rows: Front Parallax, Back Parallax (% of screen width), Solid (bool), Opacity `[D]`. | No | 3 (with stereo) |
| CP-D14 | Add Motion Path | toggle button | – | Draws the item's animation path: Enable (Off/On/Selected), Time In/Out (frames), Show Frames, Show Keys, Draw Scale `[D]`. | No | park (animation) |

Visual grouping `[S]`: rows CP-D01..D07, a divider, CP-D08..D13, a divider,
CP-D14.

## Assembly tab

**What it's for, in plain English:** rigging and "make this scene foolproof for someone else" settings. *Select* and *Lock* stop an item from being clicked or moved by accident (handy for a camera you've framed perfectly). The rest is for people building reusable rigs: redirect a click to a different item, choose which item the arrow keys jump to, pop up a tooltip, run a command when clicked, and decide what happens when this item is dragged onto another (the per-item version of the top-bar Drop Action). For a modeller, only Select and Lock matter much.


| ID | Label | Type | Default | Behaviour | Op | Tier |
|---|---|---|---|---|---|---|
| CP-A01 | Select | dropdown: Default, Yes, No | Default `[S]` | No = can't be picked in 3D viewports, still selectable in the item list `[D]`. | No | 1 |
| CP-A02 | Lock | dropdown: Default, Yes, No | Default `[S]` | Yes = item transforms refused; display rows (Visible, Pivot, Center, Size) also locked `[D]`. Pick: transform tools show nothing on a locked item; numeric edits greyed. | No | 1 |
| CP-A03 | Redirect | item picker, (none) | (none) `[S]` | Picking this item in 3D selects the redirect item instead `[D]`. | No | 3 |
| CP-A04 | Pick Walk Up / Down / Left / Right | 4 × item picker | (none) `[S]` | Overrides which item the arrow keys select from this one (default Up = parent, Down = child) `[D]`. | No | 3 |
| CP-A05 | Command | text with history dropdown | empty | Command run when the item is clicked in 3D `[D]`. Needs a command registry (architecture #5). | No | never/3 |
| CP-A06 | Tooltip | text | empty | Hover text for the item in the viewport `[D]`. | No | 3 |
| CP-A07 | URL | text | empty | Opened by the help key + click on the item `[D]`. | No | never |
| CP-A08 | User Tag | text | empty | Free tag string for scripts `[D]`. | No | 3 |
| CP-A09 | Drop Action: Assembly / Source / Destination | 3 × dropdown, same 12 entries as the top-bar Drop Action (DA-2) | (none) `[S]` | Per-item drop behaviour: Assembly = when dropped from presets; Source = when this item is dropped on another; Destination = when another item is dropped on this one `[D]`. Pick: a per-item value other than (none) overrides the top-bar setting. | No | 3 |
| CP-A10 | Drop Script: Source / Destination | 2 × text | empty | Script run on those drops with (source, destination) `[D]`. | No | never |
| CP-A11 | Create | text | empty | Undocumented `[?]`. Skip. | – | never |

Visual grouping `[S]`: Selection (A01–A04), Utility (A05; divider; A06–A08),
Drop Action (A09), Drop Script (A10), divider, Create (A11).

## User Channels tab

**What it's for, in plain English:** make up your own named sliders and checkboxes on an item ("Door Open %", "Wheel Spin"). On their own they do nothing; in a rigging app you wire them to other values so one slider drives many things. Without a wiring system (Operandi has none) they're just labelled storage.


- **CP-U01** Empty by default; one button, **Add User Channel...** `[S]`.
- **CP-U02** It opens a dialog: **Username** (optional display label),
  **Channel Name**, **Type** (Float, Distance, Angle, Percentage, Light
  Intensity, Time, Speed, Force, Mass, Acceleration, Pixel (integer), Pixel
  (fractional), Integer, Boolean, String, Matrix, Quaternion, Divider, Axis,
  Focal Length, Int Range, Pattern, Gradient), **Mode** (Scalar, Vector XY,
  XYZ, RGB, RGBA, UV, UVW), **Default Value**, **Use Minimum Value**, **Use
  Maximum Value** `[D]`.
- **CP-U03** Each user channel then shows as an editable row on this tab;
  Divider makes a named collapsible group with no value. Right-click >
  Edit User Channel renames it `[D]`.
- **CP-U04** A user channel does nothing by itself until linked to other
  channels (rigging) `[D]`. Op: No. Tier: 3 (pick: v1 = store and edit
  Float/Distance/Angle/Percent/Integer/Boolean/String/Divider scalars only,
  saved in the scene, no linking).

## Tags tab

**What it's for, in plain English:** sticky notes and labels attached to an item that only people and scripts read: "Comment: client wants this blue", or a tag a pipeline script searches for. No effect on the viewport or render.


- **CP-T01** Empty by default; two dropdown buttons, **Add Tags** and
  **Remove Tags** `[S]`.
- **CP-T02** A tag is a short type key plus a text value; an item can hold
  any number of tags but **one of each type** `[D]`. Add Tags lists the
  defined tag types (e.g. a multi-line Comment) plus **Add Custom Tag** and
  **Edit Tag definition** (opens the tag-definition preferences: name,
  description, tooltip, number of lines, hidden) `[D]`.
- **CP-T03** Each added tag shows as a text row (multi-line if its definition
  says so); Remove Tags lists the item's tags `[D][pick]`.
- Op: No. Tier: 3. Pick for v1: one built-in "Comment" tag type plus custom
  types; no preferences editor (types typed in place).

## Common rules

- **IC-1** Every change on these tabs is one undo step named after the
  property `[pick]`.
- **IC-2** Changing several selected items at once applies to all; mixed
  values show blank `[pick]`.
- **IC-3** All values save in the native scene file `[pick]`.
- **IC-4** "Add X" buttons flip to "Remove X" once added; removing drops
  that gadget's settings (one undo step brings them back) `[D][pick]`.
