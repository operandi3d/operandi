# Operandi UI architecture: the north star (v1, 2026-11-16)

One page. Every design decision from now on should fit this picture, even
when the piece it needs isn't built yet. Details live in `01-..04-*.md`
(this folder) and `layouts-2026-11-16/`. Proposed bible rules:
`PROPOSED-BIBLE-SECTION-15.md` (UIC-220..239).

## The picture

```
 buttons  keys  menus  pies  workbench  layouts  presets  macros  scripts  MCP
     \      |     |     |       |          |        |        |       |      /
      '-----'-----'-----'---- command line (text) ----'--------'-------'---'
                                    |
                          one command registry
                         (names, args, class, ?)
                                    |
                     run -> command record -> history log
                                    |
                           undo stack (undoable only)
```

1. **Everything is a command line.** Every action, from a click to a tool
   drop to a layout switch, runs one line of text through the one registry
   (already the rule: UIC-160..165).
2. **Every container stores lines, not code.** Buttons, key map entries,
   menu and pie entries, workbench items, layout open-commands, presets
   and macros hold command lines as text. That is what makes them
   interchangeable.
3. **Every run leaves a record.** The line actually run (all arguments
   written out), its class (undoable, UI, side effect, quiet), where it
   came from, and the outcome. The History tab is these records; the
   undo list is the undoable ones; a macro is a saved list of them.
4. **Nesting is one step.** A macro, preset, script or batch is one undo
   step with its lines underneath.
5. **Buttons ask the command.** Lit, greyed and "show only when" come from
   the command's own query and enable state, so a macro button behaves
   like a built-in one.
6. **Shipped is read-only; the user's layer sits on top.** Layouts, forms,
   key maps, macros and aliases alike.
7. **The tool pipe is the same idea for tools.** A tool is a stack of
   layers set by commands; a preset is a saved stack, run by `tool.set`.

## Where each piece lands

| Piece | Lands in | Status |
|---|---|---|
| Command registry, Modo names, queries, click-only backfill | extensibility Phase 0 | mostly done |
| Rule: containers store lines (no new hard-wired buttons/keys) | **now** (bible) | rule only |
| Command classes: add side effect + quiet | **H** | small |
| Command records, history log, undo steps keep their line | **H** (between layout stages 2 and 3) | not started: **first build step** |
| Command History panel (Undos, History, Commands, Results) + command entry | **H** | not started |
| Layout file with a Command History panel kind | layout stage 1 | planned |
| Toolbox as forms holding lines; lit/greyed from queries | layout stage 3 | planned |
| Workbench: pick lines from history, add macro, add tool pipe | layout stage 4 | planned |
| Key map: key + context → any command line | layout stage 5 | planned |
| Menus/pies as forms; Form Editor | layout stage 7 | planned |
| Tool presets as data rows; Tool Pipe panel | tool-presets stages 1–3 | in progress |
| Macro recorder, `.lxm` files, stored macros, `@run`, aliases, Repeat | **M** (any time after H + stage 3) | later |
| Python scripts, user values | extensibility Phase 2 | later |
| Kits, startup commands, conditional commands | extensibility Phase 3 | later |

Stage names used by the bible rules: **now**, **H** (history), **L1..L7**
(layout stages), **M** (macros), **P** (Python and kits).

## The first step

1. Jason OKs the bible section (rules only, no code).
2. Build **H**: command records + history log + the basic Command History
   panel, with the typed command line moving into it. Nothing else changes
   for the user except a new panel that shows what they did. This unlocks
   macros, the workbench and key mapping later.
