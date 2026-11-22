# Tool presets: the recipe table (spec v0.2, 2026-11-15)

Behaviour-only, builder-safe. REA-assisted: no. No bridge pass.
Sources: the reference's user guide (local copy, paraphrased) and its
human-readable tool-preset settings. Background and the full button list:
`/mnt/project-files/tool-families-2026-11-15/TOOL-FAMILIES.md`.
Tags: **[ref]** = documented or read from plain-text settings, **[OP]** =
Operandi's own proposal, **[Q]** = open question (section 9).
Rules are **TP-xx**.

## 1. Why

In the reference, nearly every modelling button is a **recipe**: "stack these
pipe layers, then run this tool, with these values changed". Soft Move is
Move plus a Radial falloff. Thicken is Smooth Shift with four values changed.
Flex is an action centre, a falloff and Transform. **[ref]** (the guide says
outright that this is how all of its tools are built).

Operandi already has some recipes, but they are written by hand in code:
the deform buttons go through one hard-coded "activate deform preset" path,
and named tool states exist only for Bevel and Smooth Shift.

This spec replaces that with **one table of recipe rows**. A new preset
button becomes a row of data, not new code. The same table later holds the
user's own saved presets and each tool's "Tool States".

## 2. Scope and stages

- **Stage 1 (this build).** The table, the activation and drop rules, the
  factory rows for every preset button that exists today (migrated with no
  change in behaviour), plus three new rows: **Flex**, **Flare**,
  **Flip It / Flip It Origin**.
- **Stage 2.** The user saves and loads their own presets (section 7) and
  the shared Tool States section (twist-bevel spec section 7, TB-Q09).
- **Stage 3 (later, per Jason 11-14).** A user-facing Tool Pipe panel that
  shows the current stack. Not in this spec beyond leaving room for it.

## 3. Words

- **Pipe slot**: one kind of layer in the tool pipe. Slots, in evaluation
  order **[OP, matches the project's pipe rule]**: Action Centre, Action
  Axis, Falloff, Snapping, Constraint (background constraint), Placer (where
  things go: line, grid, curve, pen path …), Content (a Profile), Brush,
  then the **Actor** (the tool itself). The reference groups its layers the
  same way **[ref]**. Symmetry stays global and is never set by a row
  **[OP]**.
- **Layer**: a filled slot, for example "Falloff: Linear, Shape Smooth".
- **Row**: one recipe. Holds its layers and its actor, each with values.
- **User layer**: whatever the user had in a slot before the row was
  activated (for example the falloff they picked from the pie menu).

## 4. What a row holds

- **TP-01** **Id**: the script name. Use the reference's preset name so
  scripts port (command-names decision of 11-06): `xfrm.magnet` (Soft
  Move), `xfrm.shear`, `xfrm.taper`, `xfrm.pole` (Bulge), `xfrm.twist`,
  `Swirl`, `Flare`, `Flex`, `Thicken`, `ElementMove`, `DragWeld` (Weld) …
  `tool.set <id> on` activates it **[ref]**. Existing Operandi names stay as
  aliases.
- **TP-02** **Label, icon, key, tooltip, toolbar group.** Shown the same way
  as any tool button.
- **TP-03** **Layers**: an ordered list. Each entry is slot + layer kind +
  values to set. Values not listed take that layer's factory values.
- **TP-04** **Actor**: the tool + values to set. Values not listed take the
  tool's factory values.
- **TP-05** **Component variants** (optional): rows can come in a family
  picked by the current selection mode, written `*.Slide`, which runs
  `vert.Slide`, `edge.Slide`, `poly.Slide` or `item.Slide` **[ref]**. One
  button, several rows.
- **TP-06** **Flags**:
  - *Auto Activate*: handles appear when the button is pressed, with no
    viewport click first. On for transform and deform rows, off for
    creation tools (Cube, Bevel) **[ref]**. Each row keeps whatever its
    tool does today.
  - *Lock* (Stage 2): if on, every activation starts from the row's values.
    If off, the next activation starts from the values the user last
    finished with **[ref]**. Factory rows: off, which matches how tools
    behave today **[OP]**.
  - *Auto-Drop* (later): each stroke or drag is a new operation, as Mesh
    Paint does **[ref]**. Not needed for Stage 1 rows.
  - *Select Through* (later): a click away from the handles selects instead
    of hauling **[ref]**. Not needed for Stage 1.
- **TP-07** **Kind**: *factory* (shipped, read-only) or *user* (Stage 2).
  A user row with the same id as a factory row takes precedence. Deleting
  that user row brings the factory row back **[OP]**.

## 5. Activating and dropping

- **TP-10** Pressing a row's button: for each slot the row names, remember
  the user layer (or "empty"), then put the row's layer there. Slots the
  row does **not** name keep the user layer. **[ref]** (the reference's
  "load preset" works this way: loaded parts win, everything else stays.)
  Example: Soft Move names a falloff but no action centre, so it uses
  whatever action centre the user already has.
- **TP-11** Activating a row is not an undo step. The tool's result is, as
  today.
- **TP-12** Dropping the tool (Q, Esc, Space, picking another tool, switching
  selection mode where that drops tools today) puts back every remembered
  user layer, including putting back "empty". This is what the deform
  presets already do with falloffs; the rule now covers every slot.
- **TP-13** Going from one row straight to another: restore the first row's
  user layers (TP-12) **before** applying the second (TP-10). Rows never
  stack on top of each other.
- **TP-14** If the user changes a slot while a row is active (for example
  picks a different falloff from the pie), the change applies at once. On
  drop the slot is still restored to the user layer from before activation
  **[DECIDED Jason 11-15, TP-Q1]**.
- **TP-15** **Reset** on a row's tool goes back to the **row's** values,
  not the bare tool's. Example: Reset while Thicken is active gives Thicken's
  values, not Smooth Shift's. This is already true for Thicken today; it
  becomes the general rule.
- **TP-16** `tool.attr <id> <attr> <value>` and the Tool Properties panel
  edit the active row's actor and layers as they edit any tool today.
- **TP-17** The falloff fit-on-activation rule (FO-04) still applies when a
  row brings a geometric falloff: it fits to the selection, or to all
  geometry when nothing is selected.
- **TP-18** Tool Properties shows the stacked layers above the tool, in slot
  order, as falloff batch 1 already does for falloffs. A row adds no new
  panel layout.

## 6. Stage 1 factory rows

### 6a. Migrate (no change in behaviour)

Each row below replaces a hard-coded path. Its behaviour must stay exactly
as it is on main. Values come from the reference's preset settings
**[ref]**. Falloff Shape names follow the documented order Linear, Ease-In,
Ease-Out, Smooth, Custom (FO-30).

| Button | Id | Layers | Actor and values |
|---|---|---|---|
| Element Move | `ElementMove` | Action Centre: Element. Falloff: Element | Transform, move only |
| Soft Move | `xfrm.magnet` | Falloff: Radial, Shape Linear | Transform, move only |
| Shear | `xfrm.shear` | Falloff: Linear, auto-fit, Shape Linear | Transform, move only |
| Taper | `xfrm.taper` | Falloff: Linear, auto-fit, Shape Linear | Transform, scale only, scale handles |
| Bulge | `xfrm.pole` | Falloff: Radial, Shape Smooth | Transform, scale only |
| Twist | `xfrm.twist` | Action Axis: Auto. Falloff: Linear, auto-fit, Shape Linear | Rotate, angle 0 |
| Swirl | `Swirl` | Action Centre: Auto. Action Axis: Auto. Falloff: Radial | Rotate |
| Push | `Push` | (none) | Push, distance 0 |
| Thicken | `Thicken` | (none) | Smooth Shift: Thicken on, Shift 0, Scale 1, Max Smoothing Angle 90.1° (1.5725 rad) |
| Weld | `DragWeld` | Snapping: Element, snap to points, 15 px | Drag point |
| Quick Align, Ground Align | `QuickAlign`, `GroundAlign` | as today | as today |
| Move / Rotate / Scale / Transform / Uniform Scale | `TransformMove`, `TransformRotate`, `TransformScale`, `Transform`, `TransformUScale` | (none) | Transform with the matching handle switches |

Any other Operandi button that is a recipe today (for example the Smooth
Shift / Thicken named tool states) moves into the table in the same build.
The builder lists what it found in the hand-back.

### 6b. New rows

| Button | Id | Layers | Actor and values | Notes |
|---|---|---|---|---|
| **Flex** | `Flex` | Action Centre: Selection Border. Action Axis: Selection. Falloff: Selection | Transform: move, rotate and scale handles all on | Deform tab, Move Tools group. Key Shift+Y **[ref]**. Tutorial gap rank 6. All three layers are built. |
| **Flare** | `Flare` | Action Centre: Auto. Action Axis: Auto. Falloff: Linear | Push | Deform tab, Scale Tools group, beside Bulge **[ref]**. |
| **Flip It** | `FlipIt` **[OP]** | Action Centre: Selection | Scale −1 on one axis, then flip the polygons' facing | Polygon tab, Commands. One-shot: runs and finishes, no handles. Axis X/Y/Z chosen in a small dialog, starting on X **[DECIDED for now, Jason 11-15, TP-Q3]**. |
| **Flip It Origin** | `FlipItOrigin` **[OP]** | Action Centre: Origin | same as Flip It | Same dialog. |

Flip It notes: the reference flips the selected polygons onto the other side
of an axis **[ref]**. Mirrored polygons would face inward, so the facing is
flipped as well, keeping the surface pointing out **[OP]**. Symmetry is
ignored for both, because flipping across the symmetry plane is the point
**[OP]**. One undo step each.

## 7. Stage 2 preview: saving your own (not in this build)

Listed here so Stage 1 leaves room for it.

- **TP-30** "Save Preset" captures the current pipe (every filled slot and
  the active tool, with their values) as a user row, asking for a name
  **[ref]**. The tutorial saves "My Flex" and "My Flex No Fall" this way
  (gap K63).
- **TP-31** "Load Preset" applies a saved row by TP-10 **[ref]**.
- **TP-32** User rows live in user config, one small text file per row,
  survive restarts, and show in the same places as factory rows (tool
  button pop-up, script `tool.set`, later the preset browser) **[OP]**.
  File shape: readable text, so users can share them **[OP]**.
- **TP-33** **Tool States** (per-tool saved values, twist-bevel spec
  section 7) are user rows with one actor and no layers. Same file, same
  list.

## 8. Tests the builder should add

- Every migrated row: the result after activate → drag → drop matches main
  for the same input (golden-mesh comparison).
- TP-10/12: with a user falloff set, activate Soft Move, drop, check the
  user falloff is back. Repeat with "no falloff" (it must come back empty).
- TP-13: Soft Move → Shear directly, then drop; the user's layers are back,
  with nothing left over from either row.
- TP-15: Reset under Thicken gives Thicken's values.
- Flex on a selected cap: rotate bends the cap and fades over the
  Selection falloff steps; the unselected far end stays put.
- Flare on a cylinder: the Linear falloff end gets the full push and the
  start gets none.
- Flip It on a selection right of X = 0: polygons end up mirrored to the
  left, facing out, one undo step.
- `tool.set xfrm.magnet on` from a script gives the same result as the button.

## 9. Questions

Decided by Jason 2026-11-15: TP-Q1 restore on drop; TP-Q3 proposed dialog is fine for now;
TP-Q4 readable data file inside the app. TP-Q2 still open (Jason will confirm); builder
keeps today's tool behaviour until then.

- **TP-Q1** (DECIDED: restore on drop) A slot changed by the user while a row is active: restore on drop
  (proposed), or keep the user's change? The reference's behaviour was not
  observed. A 2-minute bridge check (needs Jason's "go") would settle it.
- **TP-Q2** Lock off for factory rows means a row remembers the last values
  the user finished with (TP-06). Confirm that's how Operandi tools behave
  today; if they always reset, keep that instead and treat Lock as Stage 2.
- **TP-Q3** (DECIDED for now: dialog) Flip It's axis: dialog, or three buttons (X/Y/Z) in a pop-up?
  The guide shows an axis choice but not its form. Proposed: a dialog with
  one axis choice, starting on X.
- **TP-Q4** (DECIDED: readable data file in the app) Rows ship as a built-in data file the app reads at start, or as
  a table compiled into the app? Proposed: a readable data file inside the
  app, the same format user rows use (TP-32), so factory and user rows are
  one code path. Builder's call if this is awkward.

## 10. Bible rules touched

- UIC-55 (value presets) and UIC-103 (panel presets) are the pattern the
  Stage 2 menus follow. No new UI in Stage 1 beyond three new buttons that
  use the existing toolbox button style.
- No deviation from the bible is proposed.
