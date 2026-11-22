# Layouts v1.4, stage 3: toolboxes as forms

From research, 2026-11-16. Behaviour only.

## Rules

- **LAY-80** A toolbox is a form: every toolbox set (Model, UV...) is a form,
  a data file listing controls, not code. A form holds, in order: command
  buttons, each one command line (e.g. "set the Relax tool", "Fit UVs"), with
  label, tooltip and icon; sub-forms (a sub-tab or a section is just another
  form placed here); dividers, optionally with a label, which make a
  collapsible section. Form settings: style (vertical toolbar, horizontal
  toolbar, property sheet, menu...), icon mode (icon, text, both) and icon
  size, and whether it can be shown in a panel ("exported").
- **LAY-81** Toolbox sets are tabs of forms. The left column is a tab group;
  each tab shows one toolbox form. The same tab list is reused by every layout
  that has a toolbox; a layout only picks which tab is open. Editing the UV
  toolbox changes it in every layout that shows it. (Note: Jason chose
  separate side columns per layout - that's panel arrangement, not forms;
  forms stay shared.)
- **LAY-84** Head and tail slots: every form has a head and a tail slot.
  Another form can put itself at the start or end of any form by naming that
  slot, with a sort number for order. The target form is never edited.
- **LAY-85** Show only when: a form or section can name a command; it shows
  only while that command is enabled.
- **LAY-86** Proficiency filter: each control has a level (Basic, Standard,
  Advanced); a panel can show only up to a level. (Proficiency is already
  scaffolded.)
- **LAY-87** User changes sit on top: shipped forms read-only; user's edits
  stored separately and win; Revert drops them.

Builder note: forms reuse the one shared form component (UIC-98, UIC-125); a
form file is JSON in the same Layouts data folder; command buttons use
registry command names (UIC-160..165).

## UI conventions that apply

- **UIC-197** One toolbox panel, several toolbox sets. Each layout opens with
  its own set; set buttons above the toolbox switch only the toolbox. Click =
  show, Alt+click = popover, Ctrl+click = palette.
- **UIC-210** Every toolbox, toolbar and menu is a form: a data file of
  command buttons, sub-forms and dividers. Adding a tool button means editing
  data, never code.
- **UIC-211** Toolbox sets are tabs of forms, shared by every layout; a layout
  picks the open tab.
- **UIC-214** Head/tail slots as LAY-84.
- **UIC-215** Show-only-when + proficiency as LAY-85/86.
- **UIC-216** Shipped forms read-only; user edits separate, win, revertable.
- **UIC-221** (applies from L3 for toolbox) Every container stores a command
  line as text; loaded lines are checked against the registry; an unknown
  line shows red and does nothing, never crashes.
- **UIC-222** (from L3) A form may hold any registry line, including user
  aliases and @macro; no allow-lists.
- **UIC-223** (from L3) A button's lit state comes from its command's `?`
  query answer, its greying from the command's enable state (reason as
  tooltip), and show-only-when from a named command's enable state. No
  separate UI-side logic for these. If a command lacks a query/enable answer
  it needs, add it to the command, not the UI.

Every new registry command needs a class (undoable / UI / side effect /
quiet).
