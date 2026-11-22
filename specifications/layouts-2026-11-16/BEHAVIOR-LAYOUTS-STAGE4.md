# Layouts v1.4, stage 4: workbench edit mode

From research, 2026-11-16 (relayed by the coordinator). Behaviour only.

## Scope

Stage 4. Workbench edit mode. Edit Workbench, Pick to Add, reorder,
right-click properties, dividers with labels, add the current tool pipe as a
preset; an empty workbench tab in the toolbox (LAY-82). User edits over
shipped forms, with Revert. Rule landing: UIC-212.

## Rules

- **LAY-82** Workbench: edit in place. A workbench is a form the user edits
  right where it sits.
  - An Edit Workbench button shows the edit tools.
  - Pick to Add: click any button, property, preset, or a line in the command
    history, and it is added. Items under the pointer light green while
    picking.
  - Drag to reorder. Right-clicking an item offers: Label, Tooltip, popup
    face, alignment (normal, wide, full width), proficiency level, the
    command itself, toggle style (default, checkbox, toggle button), custom
    icon from disk, Duplicate, Delete, Show in Form Editor (greyed: the Form
    Editor is stage 7).
  - Add Divider and Add Divider with Label make a collapsible section, which
    can't collapse while editing.
  - Add the current tool pipe as a saved preset (a custom tool in one click),
    a recorded macro, or a script.
  - Any form can be made editable this way (a "Customizable Workbench"
    setting; the editor UI for that is stage 7, so for now it's a form-file
    flag). The shipped layouts have an empty workbench tab in the toolbox and
    one along the top.
- **LAY-87** User changes sit on top: shipped forms are read-only; the user's
  edits are stored separately and win. Revert drops them.
- **LAY-86** Controls carry a proficiency level (Basic/Standard/Advanced)
  that a panel can filter by.

## UI rule landing

- **UIC-212** Workbench: Edit Workbench, Pick to Add (click any button,
  property, preset or command-history line), drag to reorder, right-click to
  set label, tooltip, icon, width, style, proficiency; dividers with labels
  make collapsible sections; add the current tool pipe as a preset. Any form
  can be made a workbench.

## Build rules

- Nothing changes for the user until they open the Workbench tab or Edit
  Workbench; the rest of the app looks identical (paint-identical check on
  the Model toolbox tabs and the top bar).
- If the "along the top" workbench needs a layout-bar change, keep it an
  empty, slim, hideable row and log it as provisional.
- A macro/script entry may be greyed with a "not in Operandi yet" tip if
  there's no macro recorder yet; log that.
