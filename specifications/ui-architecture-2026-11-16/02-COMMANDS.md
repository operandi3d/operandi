# Commands: classes, lines, queries, aliases (v1, 2026-11-16)

Behaviour only. REA-assisted: no. Sources as in `01-DESIGN-PHILOSOPHY.md`.
Tags: [ref] the reference does this; [OP] Operandi's choice; [?] not
confirmed, see questions. Builds on bible UIC-160..165 and the name map
(`command-names-2026-11-06/`). Rule IDs **CMD-xx**.

## 1. Command classes

- **CMD-01** Every command has one class [ref]:
  - **Undoable**: changes the scene; one step on the undo stack; listed in
    the undo list and the history.
  - **UI**: changes only the interface (layout, panel, view option, form
    edit, workbench edit). Not on the undo stack; listed in the history;
    hidden from the undo list unless the user turns on "Show UI Only
    Commands in Undo List".
  - **Side effect**: acts outside the scene and can't be undone (save,
    export, quit, start/stop macro recording, clear undos). In the history;
    hidden from the undo list unless "Show Side Effect Commands" is on.
  - **Quiet**: internal plumbing (queries fired by buttons drawing
    themselves, recents lists). Only in the history tab, never in the
    undo list.
- **CMD-02** Undo and Redo themselves are not undo steps; they move the
  pointer in the undo list. [ref]
- **CMD-03** Operandi registry mapping [OP]: `undoable` → Undoable,
  `ui_only` → UI; add `side_effect` and `quiet` flags. A command with none
  of them must pick one (registry test fails otherwise), so the history
  always knows how to show it.

## 2. A command line

- **CMD-10** Shape: `name arg arg name:value ...`. Arguments may be given
  in order or by name; strings with spaces are quoted. Names and argument
  names follow the name map (UIC-162). [ref]
- **CMD-11** A value given in square brackets may carry units
  (`[50 %]`, `[30 deg]`); a bare number is in script units (fractions,
  radians) (command-names decision 11-07). [ref]
- **CMD-12** **Missing arguments open the command's dialog** with the
  current values filled in; giving them all runs straight through
  (UIC-80, UIC-162). [ref]
- **CMD-13** Prefixes on a line [ref for `!`, `!!`, `@`, `?`]:
  - `!` runs without the dialog (uses current values for missing args);
  - `!!` as `!` and also suppresses error pop-ups (failure still goes to
    the Results log and the history);
  - `+` forces the dialog even when all arguments are given [?: not in
    the local docs; confirm with ARC-Q2's bridge pass];
  - `@` runs a macro or script (`@name args`, MAC-30).
- **CMD-14** **Recorded as fired.** The reference records a line exactly
  as it was fired (probe 11-16), and a tool's values as one `tool.attr`
  line per changed value. Operandi does the same for tools. For a command
  that opened its dialog, the line recorded is the one *after* the dialog,
  with the chosen values written out, so a replay never depends on
  remembered dialog values. [ref + OP]

## 3. Queries

- **CMD-20** A `?` in place of an argument's value asks for that value
  instead of setting it; the answer goes to the Results tab and to the
  caller (script). [ref]
- **CMD-21** Booleans: a toggle button is a boolean command; its lit state
  is the query answer. [ref]
- **CMD-22** **Enable state**: every command can answer "can I run now?"
  (with a reason when not). Greyed buttons, hidden form sections ("show
  only when", LAY-85) and user-value enable rules all use it. [ref]
- **CMD-23** Queries are quiet (CMD-01): asking never adds an undo step
  and never shows in the undo list. [ref]

## 4. Aliases (user-named commands)

- **CMD-30** The user can create an **alias**: a new command name that
  stands for a command line with some arguments filled in
  (`cmds.aliasCreate alias:<name> source:<command> args:<args>`); and
  delete it (`cmds.aliasDelete`). An alias then works everywhere a command
  does: typed, keys, buttons, forms, macros. [ref]
- **CMD-31** Operandi's built-in aliases (old names, friendly phrases)
  already exist in the registry; user aliases use the same table, stored
  in the user layer (ARC-17), and marked as user-made in the command list.
  [OP]
- **CMD-32** An alias may not replace a built-in command's name. [OP, DEV-3 approved 11-16,
  safety; the reference's docs describe aliases that can replace
  commands, but this is an expert feature Operandi doesn't need yet.]

## 5. Batches and blocks

- **CMD-40** `cmds.batch label:<text> c1:<line> c2:<line> ...` runs up to
  many lines as **one undo step** under one label. [ref, by name and
  arguments in the command list; behaviour [?]]
- **CMD-41** Scripts can open and close a command block so everything
  inside is one undo step (Operandi's Python `undo_group`, extensibility
  plan). [ref: block begin/end exist in the scripting API]
- **CMD-42** `app.undoSuspend` (scripts only): undos are thrown away
  while the script runs, and the whole undo stack is cleared, to save
  memory on huge jobs. [ref] Operandi: later, with Python.

## 6. Startup commands

- **CMD-50** A user list of command lines runs at the end of launch, in
  order (a "late" list runs after everything else has started). Lines can
  be `@macro` or scripts. [ref] Operandi: later; one list in the user
  layer.

## 7. Help and the command list

- **CMD-60** Every command has a user name, a description, and per
  argument a user name and description, shown in the Command History's
  Command List tab and by `cmds.showHelp <command>`. [ref] Operandi:
  already has `cmds.showHelp`, `command.list`; argument help comes from the
  registry's `about` text.
- **CMD-61** `cmds.saveList` writes every command to a text file. [ref]
