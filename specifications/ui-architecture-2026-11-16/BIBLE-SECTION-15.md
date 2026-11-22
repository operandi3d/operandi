## 15. Command history and macros

Added 2026-11-16, Jason's OK ("Add it"). Detail: `ui-architecture-2026-11-16/` (ARCHITECTURE.md, HIS/MAC/CMD rules). Built in stages; each rule says when it starts to bind.
Until then, UIC-220 applies.

Stages: **now**; **H** = the history build (command records, history log,
Command History panel; between layout stages 2 and 3); **L1..L7** =
layout stages (section 14); **M** = macros; **P** = Python and kits.

### The bridge rule

- **UIC-220** **Don't block the future.** Until a rule below applies, new
  work must not make it harder: no new button, key, menu, pie or panel
  action that calls code directly instead of running a command line; no new
  undo path that can't say which command made the step; no new interactive
  tool session with its own private undo that couldn't emit command lines.
  A builder who can't avoid it lists it in FLAGS.md with the rule number.
  *Applies from: now.* [OP]

### Containers hold lines

- **UIC-221** Every container stores a **command line as text**: form
  controls (toolbox, menu, pie, workbench), key map entries, layout
  open-commands, preset rows, macros. Loaded lines are checked against the
  registry; an unknown line shows red and does nothing, never crashes.
  *Applies from: now for new containers; L3 toolbox, L5 key map, L7 menus
  and pies.* [ref]
- **UIC-222** A container may hold **any** registry line, including a user
  alias and `@macro`. No allow-lists of "tool buttons" or "key actions".
  *Applies from: L3 (forms), L5 (keys).* [ref]
- **UIC-223** A button's lit state comes from its command's `?` answer, its
  greying from the command's enable state (with the reason as the
  tooltip), and "show only when" from a named command's enable state. No
  separate UI-side logic for these. *Applies from: L3.* [ref]

### Records and history

- **UIC-224** Every run produces a **command record**: the line **as
  fired** (Modo names; after a dialog, the line with the chosen values),
  class, source (typed, key, button, menu, pie, script, macro, bridge),
  success or failure message, sub-commands, time. Tool sessions record
  `tool.set <tool> on`, one `tool.attr` line per changed value,
  `tool.doApply`, `tool.set <tool> off`; a live drag records the final
  values at mouse-up, never mouse moves. Selections record by element
  index. *Applies from: H.* [ref, bridge probe 2026-11-16]
- **UIC-225** Every command has a **class**: undoable, UI, side effect or
  quiet. Only undoable commands make undo steps. Queries are quiet. The
  registry test fails if a command has no class. *Applies from: H.* [ref]
- **UIC-226** Each undo step keeps the record(s) that made it, as well as
  its friendly label. Every history owner (document, live tool, extrude
  stage, primitives, pen, sketch, reduction …) does this. *Applies from:
  H.* [OP, needed for ref behaviour]
- **UIC-227** A command that runs commands (macro, preset, script, batch,
  conditional command) is **one undo step** with its lines as sub-commands.
  *Applies from: H.* [ref]
- **UIC-228** The **Command History** panel: tabs Undos, History,
  Commands, Scripts, Results, with a filter per tab and the one-line
  command entry at the bottom; collapsed to that line in the Model layout.
  Behaviour per `03-COMMAND-HISTORY.md` (HIS-xx). *Applies from: H (Undos,
  History, Commands, Results); M (Scripts).* [ref]
- **UIC-229** Undo depth pref **Max Undo Levels** (default 100); **Group
  Selection Commands** and **Max Sub-Commands Recorded** as in the
  reference. *Applies from: H.* [ref]

### Using the history

- **UIC-230** Right-click a command (Commands or History tab) → **Map
  Command to Key**, with clash warning and a context picker starting at
  Global. *Applies from: L5.* [ref]
- **UIC-231** Workbench **Pick to Add** accepts a line picked from the
  Command History. *Applies from: L4.* [ref]
- **UIC-232** **Fire Again** repeats the last command, optionally skipping
  UI and selection commands. *Applies from: M.* [ref]

### Macros

- **UIC-233** A macro is a plain list of command lines with `%1`-style
  arguments, run as one undo step. If a line fails or Esc is pressed, the
  whole macro is undone and the message names the line (DEV-4, approved
  11-16). *Applies from: M.* [ref + OP]
- **UIC-234** Recorder at the right of the command entry: Record/Stop,
  Replay (hold for Replay Repeat), Options (save to file, store, record UI
  commands: Automatic (default), Always, Never). Only top-level lines are
  recorded (a script is one line); queries and quiet commands never; Undo
  removes the undone lines from the recording. *Applies from: M.* [ref,
  bridge probe 2026-11-16]
- **UIC-235** Macro files use the reference's `.lxm` text format
  (`#LXMacro#` first line, `#` comments). Stored macros are files in the
  user Macros folder (DEV-2, approved 11-16); shipped and kit macros are
  read-only (lock). *Applies from: M.* [ref + OP]
- **UIC-236** A macro runs from anywhere a line is accepted with
  `@<name> args`. "Save a macro as a command" = a **user alias** naming it;
  aliases can't take over a built-in name (DEV-3, approved 11-16).
  *Applies from: M.* [ref + OP]
- **UIC-237** Scene files never hold macros or scripts that run on open
  (extensibility D5). *Applies from: now.* [OP]

### Layers

- **UIC-238** Macros, aliases, key maps and forms follow the layouts'
  storage rule: shipped read-only, user layer on top, Revert drops the
  user copy (UIC-194). *Applies from: as each lands.* [ref]
- **UIC-239** Every GUI QA checklist that adds a control includes: "the
  action shows in the Command History with its line". *Applies from: H.*
  [OP]

---

