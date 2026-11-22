# Set Part, behaviour spec v1

Provenance: Modo 17.1 user guide and plain-text UI config (labels, menus,
option lists). Not observed in Modo. REA-assisted: no.

## What it is

A **Part** is a named label stored on polygons. It groups polygons for
surfacing masks and for export, and is one of four polygon tag kinds:
**Material**, **Part**, **Smoothing Group** and **Selection Set**. Set Part
writes the Part label and nothing else.

## The tag model

- **SP-01** Each polygon has at most one Part. Assigning a new Part replaces
  the old one. (Same rule as Material. Selection Sets are different: a
  polygon can be in many sets.)
- **SP-02** Part is independent of the other kinds. Setting a Part never
  changes a polygon's Material, Smoothing Group or Selection Sets, and they
  never change its Part.
- **SP-03** Part names are free text, kept exactly as typed (case and
  spaces). The same name on two meshes is the same Part for listing and
  selecting.
- **SP-04** Unlike Set Material, Set Part creates **nothing** in the shader
  tree or anywhere else in the scene. It only labels polygons. (A material
  group can later be told to mask by a Part; that belongs to the future shader
  tree, not to this tool.)

## Which polygons

- **SP-05** Polygon mode with polygons selected: only the selected polygons
  are tagged. Polygon mode with nothing selected: **every polygon in the
  active mesh(es)** is tagged. This is the documented rule for the shared
  Material/Part/Smoothing family; see SP-Q01.
- **SP-06** If several meshes are active, the rule applies in each of them,
  and it is one step.
- **SP-07** Curves selected while in Polygon mode can be tagged too, the same
  way as polygons (later slice; today's refusal of line polygons can stay
  until then).
- **SP-08** Other selection modes: see SP-Q02. Provisional: keep today's
  refusal with a clear message ("Set Part works in Polygon mode").

## The dialog

- **SP-09** Title **Polygon Set Part**. One row: label **Name**, a text
  field, and a small dropdown button right of the field. Then **OK** and
  **Cancel**. No other fields (this is the main difference from Set
  Material's dialog, which also has Type, Color, Diffuse, Specular,
  Smoothing, Make Default, Add to Library).
- **SP-10** The field opens holding the **Default Part** preference value
  (SP-17). See SP-Q03 for whether it remembers the last name instead.
- **SP-11** The dropdown lists every Part name already used anywhere in the
  scene. Picking one fills the field; it does not apply anything.
- **SP-12** OK or Return applies the name to the target polygons and closes
  the dialog. Cancel, Escape or closing the window applies nothing and leaves
  no undo step.
- **SP-13** Blank name: see SP-Q04. Provisional: keep today's behaviour (OK
  disabled, short message).

## Where it lives

- **SP-14** Toolbox, Polygon tab, **Commands** group: button **Set Part...**
  placed after **Align** and **before** **Set Material...**. No icon, text
  button like its neighbours.
- **SP-15** Menu: **Geometry > Polygon > Set Part...**, after **Assign
  Material Group...** and before **Set Smoothing Group...**. No default
  keyboard shortcut. Tooltip in own words, e.g. "Give the selected polygons a
  part name". Command line: `poly.setPart <name>` applies directly; bare
  `poly.setPart` opens the dialog (already built).

## Files

- **SP-16** Parts survive save/load in the native format (already built for
  LXO). Later: OBJ export writes each Part as a group (`g`) when an export
  option "Save Meshes as Groups" is on, and as objects (`o`) otherwise; FBX
  has "Save Polygon Parts" / "Load polygon parts" options; Alembic can export
  parts as face sets. Build only what Operandi's exporters already have room
  for; list the rest.

## Preferences

- **SP-17** Preferences > Defaults (next to **Default Material**): text field
  **Default Part**, tooltip "Name given to new parts". It sets the name the
  dialog opens with. Default value: SP-Q05 (provisional: `Default`, matching
  Default Material).

## Shared with Set Material and the Select tab

- **SP-18** One polygon-tag model with four kinds: Material, Part, Smoothing
  Group, Selection Set. Material and Part are single-valued per polygon,
  Selection Set is multi-valued, Smoothing Group is a set of characters (see
  the smoothing spec). Store Part next to Material; do not invent a second
  per-face store.
- **SP-19** One shared helper "all names of tag kind K in the scene", used
  by: the Set Part dropdown, the Set Material dropdown, and the Select tab's
  **Polygon Tag** dropdown. One shared "which polygons" rule (SP-05, SP-06)
  for Set Part and Set Material. One shared Name-field-plus-dropdown dialog
  row, so both dialogs behave the same (focus, Return, Escape, dropdown).
- **SP-20** Selecting by tag: the Select tab's **Polygon Tag** dropdown
  chooses the kind (**Material**, **Part**, **Smoothing Group**, **Selection
  Set**; default Material). Selecting by a Part name selects every polygon
  carrying it; a matching "deselect by Part name" also exists. With no parts
  in the scene, a Part list shows a short message such as "No parts exist."
  The selecting side can be its own build; Set Part must only expose the
  names and per-polygon lookup it needs.

## Undo

One undo step per apply, named after the tool (Edit menu reads "Undo Set
Part", per Jason's rule). Re-applying a name the target already has is a
no-op with no undo step (already built).
