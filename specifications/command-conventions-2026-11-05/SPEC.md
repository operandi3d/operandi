# Command conventions — behavior spec v1

Provenance: observed in Modo 17.1v1 through scripted probes on 2026-11-05; the Modo 17.1 user guide for option names and intent (paraphrased). REA-assisted: no.

## How a command line is read

1. **Shape:** a command name of the form GROUP.ACTION, then arguments
   separated by spaces.
2. **Named or positional:** an argument can be given as NAME:VALUE in any
   order (CV01, CV03), or by position in the command's declared order
   without names (CV02). All three give the same result.
3. **Choices** accept their name or their index (axis y = axis 1, CV04).
   An unknown choice name doesn't fail. The command reports success and
   does nothing (CV05). **Decided for Operandi (Q-CV-01):** refuse it with
   an error listing the valid choices.
4. **Booleans** accept true/false, on/off and 1/0 alike (CV08–CV10).
5. **Numbers with unit text:** the number is used and the unit text
   dropped. "50cm" became 50 (metres) and "250mm" became 250 (CV06, CV07).
   Percent settings are stored as fractions: 0.5 means 50 %, and "50%"
   became 50, i.e. 5000 % (CV15). **Decided for Operandi (Q-CV-02):**
   parse mm/cm/m and % properly and refuse any other trailing text.
6. **Text with spaces** must be quoted, with braces {Left Wing} or double
   quotes "Left Wing" (CV11, CV12). Unquoted, only the first word is used
   (CV13: "Left").
7. **Queries:** a ? in place of a value returns the current value instead
   of setting it: a tool setting's number (CV14) or a toggle state (CV16:
   1 for the active selection mode, 0 otherwise).
8. **Dialogs:** commands that normally open a dialog (Set Position, Set
   Material, Set Part, Mesh Cleanup) run straight through when every
   argument is given and dialogs are suppressed (a leading "!" in Modo).
   A query with a required argument left out can instead open the dialog
   and wait. Operandi's command line should never block like that
   (Q-CV-03).
9. **Errors:** an unknown command name fails with "command could not be
   found" (CV17); a missing required argument fails with "Missing required
   argument" plus its name (seen with Make Polygon in batch 3); a command
   used in the wrong selection mode fails with its own message (for
   example Flip Polygons from Vertex mode: "Need to directly select one or
   more polygons.").
10. **Tools vs commands:** a tool is switched on and off by name and its
    settings are set and queried one by one while it's on (CV14). Reset
    restores the defaults, and settings otherwise carry over between uses
    (bevel-activation spec).

## Open questions and decisions

- **Q-CV-01** (decided below) An unknown choice value is silently ignored and the
  command reports success (CV05). Sane rule for Operandi: refuse it with
  an error naming the valid choices.
- **Q-CV-02** (decided below) Unit and percent text is dropped ("50cm" → 50 m,
  "50%" → 5000 %). Sane rule: parse units (mm, cm, m, %) properly, or
  refuse text after a number; never silently misread it.
- **Q-CV-03** Dialog behaviour: Operandi's command line should never open
  a modal dialog by itself; missing arguments are an error.
- **Q-CV-04** Unknown argument names (CV18) and several commands on one
  line (CV20) weren't observed: Modo stopped answering during CV18.

## Decided (2026-11-05)

- **Q-CV-01 → decided:** an unknown choice value is refused with an error
  that lists the valid choices (CV05 is Modo reference only).
- **Q-CV-02 → decided:** unit text (mm, cm, m) and % are parsed properly;
  any other text after a number is refused. Never silently misread
  (CV06, CV07, CV15 are Modo reference only).
