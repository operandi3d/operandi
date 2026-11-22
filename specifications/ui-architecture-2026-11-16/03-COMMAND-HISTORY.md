# Command History panel (v1, 2026-11-16)

Behaviour only. REA-assisted: no. Rule IDs **HIS-xx**. Tags as in
`02-COMMANDS.md`.

## 1. Where it lives

- **HIS-01** A panel kind, **Command History**, placeable in any layout.
  In the Model layout it sits at the bottom right, collapsed to its
  one-line **command entry** field; dragging the divider above it upward
  reveals the tabs. [ref]
- **HIS-02** The command entry and the macro recorder strip (MAC-01) stay
  visible even when the panel is collapsed to one line. [ref]

## 2. Tabs

Five tabs, Undos first and open by default. [ref] Each tab has an **F**
button that shows a filter field for that tab; turning F off clears the
filter. [ref]

### Undos

- **HIS-10** One row per undo step, oldest at top, with columns:
  **Command** (user name), **Fired name and arguments** (the canonical
  line), **Description**. Each row's icon shows its class: undoable,
  redoable (undone but still redoable), UI, side effect. [ref]
- **HIS-11** A bold row **"Drag to undo/redo"** marks the current point.
  Drag it up to undo several steps at once, down to redo. [ref]
  (`cmds.undoTo` jumps to a clicked row. [ref, by name])
- **HIS-12** A row that ran other commands has an arrow; open it to see
  its sub-commands. Only the first few sub-commands are kept (pref **Max
  Sub-Commands Recorded**), to save memory. [ref]
- **HIS-13** Double-click a row: runs that line again. Single-click: puts
  the line into the command entry, ready to edit. [ref]
- **HIS-14** Undone steps stay listed (greyed, "redoable") until a new edit
  throws them away. [ref]
- **HIS-15** Undo depth: pref **Max Undo Levels**, default **100**. [ref]
- **HIS-16** **Group Selection Commands** pref: several selection clicks in
  a row become one undo step (grouped up to Max Sub-Commands). [ref]
  Operandi already has selection undo settings (`history_settings.rs`);
  map them to these two names.

### History

- **HIS-20** Every command ever run this session, in order, **ignoring**
  undo: mistakes that were undone, UI commands and quiet commands are all
  here. [ref] Tool use shows as: tool on, one `tool.attr` line per changed
  value, `tool.doApply`, tool off (probe 11-16).
- **HIS-21** Double-click runs the line; click puts it in the entry. [ref]
- **HIS-22** Length cap `history.max`; `history.clear` empties it. [ref]
- **HIS-23** Select several history rows → **Macro from History**
  (`macro.createFromHistory`) makes a stored macro of them;
  `attr.controlsFromHistory` turns them into form/workbench buttons. [ref,
  by name and label]

### Commands

- **HIS-30** Every command, grouped by category, with help (CMD-60).
  Double-click runs it (dialog if arguments are missing). Options to show
  user names instead of internal names. [ref]
- **HIS-31** Right-click a command → **Map Command to Key**: a small
  dialog shows the command, waits for a key or combination, shows a clash
  ("already used by …") at the bottom, and has a context picker that
  starts at **Global [Anywhere]** and can pick a panel kind or part of
  one. OK saves. [ref]

### Scripts

- **HIS-40** Lists stored macros (and installed scripts). Stored user
  macros can be renamed, described, deleted, run, mapped to a key
  (`script.mapKey`) and have single lines edited or run; shipped or kit
  macros show a **lock** and can only be run. [ref]

### Results

- **HIS-50** Every line run from this panel (typed or double-clicked) logs
  its outcome: success, failure with message, or the query's answer.
  Length cap `history.resultsMax`. A good place to try commands. [ref]

## 3. The command entry

- **HIS-60** Type a line, Return runs it; the line joins the history (and
  the undo list if undoable). [ref] Operandi's typed command line moves
  here.
- **HIS-61** Up/Down in the entry steps through earlier lines. [OP;
  common practice, not confirmed in the reference's docs]
- **HIS-62** `cmds.autoRefocusCommandEntry` keeps focus in the entry
  after running, for typing many commands. [ref, by name]

## 4. Repeat and recents

- **HIS-70** **Fire Again** (`cmds.fireAgain`) re-fires the last command
  sent by a **key, button or form** (its three modes; typed lines don't
  count). Options: allow UI commands and allow selection commands (both on
  by default); raw (on by default) re-fires exactly what the key or button
  sent, before any dialog. [ref, probe 11-16; default key unknown]
- **HIS-71** Forms can show **Recent Commands** and **Frequent Commands**
  lists (`attr.recents`, `attr.frequents`), clickable. [ref] Operandi:
  later, but the history log must keep enough to count them.

## 5. Builder notes [OP]

- The Undos tab is a **view of the undo stack**; the History tab is a
  **separate append-only log**. Both are built from command records
  (ARC-10). Neither is saved in the scene file.
- The history must not slow editing: records are small (a line and a few
  fields); sub-command lists are capped (HIS-12).
- Live drags never flood the history: one record per tool value change at
  mouse-up, not per mouse move (UIC-161).
