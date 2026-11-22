# Set Part (polygon Part tag), build hand-off, v1

Date: 2026-11-04. Status: ready to build.
Provenance: Modo 17.1 user guide (Polygon Set Material/Part/Smoothing; Defining
Shaders, Part Tags; Material Group; Selecting modes; Defaults and File I/O
preferences; Pack UVs; Merge Meshes) and Modo's plain-text UI config for
labels, menu placement and option lists. Nothing was observed in Modo for this
version. REA-assisted: no.

## Files

- `BEHAVIOR-SET-PART.md`: the behaviour spec (SP-01 to SP-20).
- `QUESTIONS.md`: open questions (SP-Q01 to SP-Q07) with a provisional answer
  for each, so the build is not blocked.
- `cases.json`: small acceptance cases on original meshes.

## Where Operandi already is

Operandi already has a basic Set Part (core operation, dialog, command
`poly.setPart <name>`, tests). This hand-off is the upgrade from "basic" to
"matches the docs". The gaps, in priority order:

1. **Nothing selected = every polygon of the active mesh(es)** (SP-05). Today
   it refuses.
2. **Name field starts from the Default Part preference** (SP-09, SP-17), not
   from the selection's shared part. Add the preference.
3. **Panel placement**: Set Part sits *before* Set Material in the Polygon
   Commands group (SP-14). An older hand-off said "after"; that was wrong.
4. **Several active meshes** get tagged in one step (SP-06).
5. **Shared tag plumbing with Set Material and the Select tab** (SP-18 to
   SP-20). This is the main thing to coordinate with the Set Material build.

Curves (SP-07) and file export of parts (SP-16) are later slices.

## Build slices

- **SP-1**: SP-05, SP-06, SP-09, SP-14, SP-17 (behaviour fixes on the existing tool).
- **SP-2**: shared polygon-tag helpers used by Set Part, Set Material and the
  Select tab's Polygon Tag dropdown (SP-18 to SP-20).
- **SP-3**: curves, OBJ/FBX part export, Statistics-style part list (SP-07, SP-16).
