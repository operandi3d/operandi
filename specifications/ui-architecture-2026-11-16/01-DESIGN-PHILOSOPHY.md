# The reference's design philosophy, and what Operandi must lock in (v1, 2026-11-16)

Behaviour only, clean room. REA-assisted: no. No bridge pass.
Sources: the reference's user guide (Command History, Workbenches, Tool
Pipe, Tool State presets, Input Editor, Preferences pages), its public SDK
documentation (Command System, Command History viewport, Macros, Form
System, Form Editor, Form Filtering, Kits, User Values, App.undoSuspend,
Startup Commands), and the command list already in
`command-names-2026-11-06/modo-commands.json` (names, labels, argument
names only). Operandi side: read-only look at main `2bfbd6ba` on 11-16,
plus `extensibility-2026-11-09/EXTENSIBILITY-PLAN.md`,
`tool-families-2026-11-15/`, `tool-presets-2026-11-15/`,
`layouts-2026-11-16/`.

Companion specs: `02-COMMANDS.md` (CMD-xx), `03-COMMAND-HISTORY.md`
(HIS-xx), `04-MACROS.md` (MAC-xx). This file holds the big picture (ARC-xx)
and the gap list.

---

## 1. The one idea: everything is a line of text

The reference's whole UI is built on one rule: **every action is a command
line** (a name plus arguments, as text). Clicking a button, pressing a key,
picking a menu or pie entry, dragging a tool and dropping it, changing a
field, switching a layout, saving a file: each one runs a command line.
[ref: SDK Command System overview; guide: "All actions in Modo are logged
there".]

Everything else is a container that **holds command lines**:

| Piece | What it really is |
|---|---|
| Button / menu entry / pie slice | a form control holding one command line |
| Toolbox, menu, pie, property sheet, workbench | a **form**: an ordered list of command lines, sub-forms and dividers |
| Layout | a data file that places panels; may name one command to run when it opens |
| Key map entry | key + context → one command line |
| Tool preset (Soft Move, Flex ...) | a saved tool pipe, activated by the command `tool.set <preset> on` |
| Tool State | saved tool values, recalled by a command |
| Macro | a list of command lines, run by one command (`@name`) |
| Alias | a new command name that stands for a command line |
| Conditional command | one key whose command line depends on the selection |
| Startup command | a command line run at launch |
| Script (Python) | a program that sends command lines and queries |
| Kit | a folder of config: forms, keys, macros, aliases, presets, scripts, icons |

Because they all speak the same language, they plug into each other for
free: a recorded macro can go on a button, a key, a pie, a workbench or a
conditional command with no special code; a line in the Command History can
be dragged into a form; a key can run a script.

**ARC-01 Operandi adopts the same rule end to end.** The bible already says
"every user action is a command" (UIC-160..165). This spec adds the other
half: every *container* (form control, key map entry, pie slice, menu
entry, layout open-command, workbench item, preset, macro) stores **a
command line as text**, never a direct call into code. That is what makes
macros, buttons and keys interchangeable. [ref]

## 2. Six supporting ideas

**ARC-02 Commands have classes, and the class decides undo and history.**
Every command is one of: *undoable* (changes the scene, goes on the undo
stack), *UI* (changes only the interface, e.g. switch layout, open a
panel), *side effect* (does something outside the scene that can't be
undone, e.g. save a file, record a macro) or *quiet* (internal, shown only
in the full history). The Command History marks each with an icon. [ref]
Detail: `02-COMMANDS.md`.

**ARC-03 Two lists, not one.** The **undo list** is the shortest set of
steps that got the scene to where it is now (undone steps drop off). The
**history** is everything that ever ran, including mistakes that were
undone and UI commands. Macros and "repeat" are built from these lists.
[ref] Detail: `03-COMMAND-HISTORY.md`.

**ARC-04 Commands nest.** A command that runs other commands (a macro, a
preset, a script, a conditional command) shows them as **sub-commands**
under one row, and the whole thing is **one undo step**. [ref]

**ARC-05 Ask and tell use the same words.** The same command reads a value
with `?` in place of the value (`tool.attr poly.bevel inset ?`). Buttons
use this to draw their state: a toggle button lights up because its
command's query answers true; it greys out because the command says it is
disabled; a form section shows only while a named command is enabled
("show only when", LAY-85). [ref: Form Filtering, User Values
enablecommand] So greying, highlighting and filtering are **not UI code**;
they are command queries.

**ARC-06 Shipped stuff is read-only; user stuff sits on top.** Forms,
layouts, key maps, macros and aliases each come from read-only shipped
files with the user's changes stored separately and winning. Only user
items can be renamed or deleted; shipped ones show a lock and can only be
reverted. Add-ons extend shipped forms through head/tail slots without
editing them (LAY-84). [ref]

**ARC-07 The tool pipe is the same idea for tools.** A tool is a stack of
layers (action centre, axis, falloff, snapping, constraint, placer,
content, actor). Each layer is set by a command, so a preset is just "a
list of layer commands", saved under a name, run by `tool.set`. The Tool
Pipe panel shows the stack and can save it as a preset; a workbench can
add "current tool pipe" as a button. [ref] Operandi already plans this
(tool-families, TP-xx); nothing new here except ARC-10.

## 3. Where Operandi stands (main 2bfbd6ba, read only)

| Piece | Reference | Operandi today | Verdict |
|---|---|---|---|
| One command list with names, args, types | yes | **yes**: `command_registry.rs` (~3,800 lines), Modo names, aliases, flags `undoable / query / ui_only / tool / headless / operandi`; click-only gaps down to ~91 (11-10) | Good base. |
| Command classes | undoable, UI, side effect, quiet | `undoable` and `ui_only` flags; no *side effect* or *quiet* | Small add (CMD-10). |
| Queries with `?` | every command | many commands answer `?` (registry stage 3 onward) | Keep filling. |
| Typed command line | bottom-right entry field | yes (app command line) | Needs to become the Command History panel's entry strip. |
| **Undo stack holds the command line** | yes (undo list = command rows, with args) | **no**: each step is a scene snapshot plus a **label** (`Editor::record(label, before)`); the line that caused it is not kept | **Gap 1, the big one.** |
| **History log of everything run** | yes | **no** | **Gap 2.** |
| Undo for live tools | one lifecycle; tool edits are tool commands | about a dozen separate history owners (document, extrude stage, cone, shapes, pen, solid sketch, reduction, live tool ...) | Each must emit command records (ARC-12). |
| Sub-commands under one undo | yes | `undo_fold` merges a gesture into one step; no parent/child record | Add with Gap 1. |
| Key map | key + context → **any command line** | `input-map.json` maps a **fixed list of action names** to keys | **Gap 3**: a key can't run a macro or arbitrary line. |
| Toolbox buttons | form controls holding command lines | `toolbox_button(tool: Tool)`: enum, hard-coded | Layout stage 3 fixes this; must store lines (ARC-09). |
| Macros (record, replay, save, store, run with args) | yes | none (Phase 1 of extensibility plan) | Build later; reserve now. |
| Aliases (user-named commands) | yes (`cmds.aliasCreate`) | registry has *built-in* aliases only | Needs user aliases (MAC-30). |
| Repeat last command | yes (`cmds.fireAgain`) | not found | Small, needs Gap 1/2. |
| Recent / frequent commands | yes (`attr.recents`, `attr.frequents`) | no | Falls out of Gap 2. |
| Results log (query answers, failures) | Results tab | typed line shows a message | Part of the panel. |
| Startup commands | config list | no | Tiny; later. |
| Conditional commands | yes | no | Later (it's how one key does Bevel/Chamfer by mode). |
| User values (script settings shown in forms) | yes | no | Later, with Python. |
| Kits | config folders | spec 05-KITS only | Later. |

## 4. Lock-ins (cost little now, a lot later)

Numbered so builders can cite them. None changes how the app looks today.

- **ARC-10 Command record.** Every executed command produces one record:
  the **canonical command line** (Modo name, every argument written out,
  script units: fractions and radians), its class, where it came from
  (typed, key, button, menu, pie, script, macro, bridge), success or the
  failure message, its sub-commands, and a time. This is the atom that
  history, undo list, macros, repeat, recents and "pick to add" all use.
- **ARC-11 Undo steps point at records.** Each undo step keeps the record
  (or records) that made it, beside today's snapshot and label. The label
  stays as the friendly text; the line is what macros replay. Snapshots can
  stay as the undo *mechanism*; this is only about what is *remembered*.
- **ARC-12 Every history owner emits records.** Tool sessions record what
  the reference records: tool on, tool values, apply, tool off (see
  HIS-20). The extrude stage, primitive, pen, sketch and reduction
  sessions do the same. A live drag records the **result** (final tool
  values), never mouse moves (UIC-161).
- **ARC-13 Nesting.** When a command runs commands (macro, preset, script,
  conditional command), the children are recorded under the parent and the
  parent is one undo step. A failure stops the parent and undoes its
  children (MAC-05).
- **ARC-14 Containers store text.** Form controls, key map entries, pie
  slices, menu entries, workbench items, layout open-commands store a
  command line string, validated against the registry when loaded (an
  unknown line shows red, like the reference's Input Editor, never
  crashes).
- **ARC-15 Enable + query for everything.** Every registry command can say
  "enabled now?" and, if it has a toggle or value, answer `?`. Buttons
  draw highlight and greying from these, so a macro or alias button greys
  and lights the same way as a built-in one.
- **ARC-16 One run path.** Typed lines, keys, buttons, macros, scripts and
  the MCP bridge all execute through the same registry entry point, so a
  macro replays exactly what a click did.
- **ARC-17 User layers.** Macros, aliases, key maps and forms use the same
  "shipped read-only + user on top" storage as layouts (LAY-75, LAY-87).

## 5. What the layout stages must reserve

Against `layouts-2026-11-16/BUILD-PLAN.md` v1.4:

| Stage | Reserve for macros and history |
|---|---|
| 1 Layout file | A **Command History** panel kind in the panel registry, with its one-line **command entry** strip at the bottom right of the Model layout (reference default: collapsed to one line; drag the divider up to see the tabs). Even before the tabs exist, the strip is where today's typed line lives. |
| 1 | `layout.restore` and other UI commands are recorded as **UI class** (in history, not in undo). |
| 3 Toolbox as forms | Every control stores a **command line** (ARC-14). Highlight and greying come from the command's query and enable state (ARC-15), so they work for macro and alias buttons too. The custom-widget escape hatch must still fire a command line on click. |
| 3 | A control can hold **any** registry line, including `@macroName` and a user alias: no allow-list of "tool buttons". |
| 4 Workbench | **Pick to Add** must accept a line picked from the Command History (`attr.controlsFromHistory`), plus **Add Recorded Macro** (as file or stored) and **Add Current Tool Pipe**. So the History panel and the macro store must exist by stage 4, even if minimal. |
| 5 Per-layout keys | Key map becomes **key + context → command line** (ARC-14), replacing the fixed action-name list; old `input-map.json` names convert to their command lines once. **Map Command to Key** from a right-click in the Command History (HIS-31). |
| 6 Palettes | The Command History can be a palette; the macro recorder strip stays with the command entry. |
| 7 Form Editor | Commands are added from the history and from the command list tab (LAY-83); macros and aliases appear in the command list like built-ins. |

Suggested order (not a new stage): build **Gap 1 + 2** (records and the
history log, ARC-10..13) and a minimal Command History panel **between
layout stages 2 and 3**, so stage 3 forms and stage 4 workbench have lines
to pick from. Macro record/replay can follow any time after; it is small
once records exist.

## 6. Deviations from the reference

DEV-2..DEV-5 APPROVED by Jason, 2026-11-16 12:34Z ("I think those four
changes are ok").

- **DEV-1 Scripting language.** The reference runs macros plus Python, Perl
  and Lua. Operandi: macros plus Python only (already decided D1, 11-09).
  Listed so it's on record; no new decision.
- **DEV-2 (APPROVED 11-16)** **Stored macros live in files, not one config file.** The
  reference keeps "stored" macros inside its XML user config. Operandi
  keeps each stored macro as its own file in a user Macros folder, shown
  in the same Scripts list with the same rename/delete/lock rules. The user
  sees no difference. Builder note, not a behaviour change; flagged because
  it changes where a power user looks.

- **DEV-3 (APPROVED 11-16)** **User aliases can't take over a built-in name** (CMD-32). The
  reference allows replacing a command with an alias; Operandi blocks it
  for now so a kit can't silently change what Bevel does.
- **DEV-4 (APPROVED 11-16)** **A failed or cancelled macro undoes itself** (MAC-03, MAC-05).
  The reference stops at the failing line and leaves the earlier lines
  done (one undo away). Operandi rolls back automatically and names the
  line. Recommended; says no to half-finished edits.
- **DEV-5 (APPROVED 11-16)** **Up/Down arrow recalls earlier lines in the command entry**
  (HIS-61). Not documented in the reference; harmless addition.

Nothing else deviates.

## 7. Open questions

ANSWERED by bridge probe 2026-11-16, see `PROBE-2026-11-16-MACRO-RECORDER.md`
(only the `+` prefix is still unchecked).

- **ARC-Q1** (bridge, needs a slot): record a Bevel with the macro
  recorder and read back the exact lines, to confirm HIS-20's
  tool-session line order and whether tool values are written as
  separate `tool.attr` lines or as `tool.set` arguments.
- **ARC-Q2** (bridge): the option values of "Record UI Commands"
  (`macro.recordUICommands mode`), and of "Fire Again"
  (`cmds.fireAgain mode`), and which key, if any, runs Fire Again by
  default.
- **ARC-Q3** (bridge): how selection clicks are recorded (by element
  index?) so a recorded macro replays selection the same way.
