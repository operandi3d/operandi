# LXO export (modelling subset)

**Status: v3 (partial).** v2 added the import side of Operandi's own files (LXE-IMP-Q01); v3 adds a bare render item to every export (LXE-16), which removes the crash risk from the declared version. Observed in Modo 17.1v1 on 2026-11-03 by opening
generated test files and Modo's own saves, with every fact read back from
Modo by script (six decimals). Byte encodings follow the import spec
`lxo-import-phase1-2026-11-05` (clean re-derivation; replaces the removed phase-1 spec), which this spec builds
on; Foundry's public file-format documentation was the background source for
that spec, and nothing here is copied from it. REA-assisted: no.

## Covered

- `BEHAVIOR-WRITER.md`: the exact chunk layout an exporter writes so Modo
  17.1v1 opens a scene of meshes with names, FACE polygons, item transforms
  (position, ZXY rotation, scale), parenting, hidden state, UV maps with
  seams, and material and part labels. Includes the layouts that crash Modo.
- `BEHAVIOR-IMPORT-PROFILE.md`: how Operandi's importer recognizes and reads
  its own exports (the LXE profile), and the round-trip requirement.
- `ORACLES.json`: 11 cases with the intended scene and Modo's exact readback,
  including two save-and-reopen round trips and a 66,049-point mesh.

## Not covered (QUESTIONS.md)

Other Modo versions, pivots, other rotation orders, instances, subdivision
polygons and settings, non-UV maps, empty meshes, LXL presets, and whether
Modo needs anything else when a file is opened by import or reference rather
than File > Open.

## For the implementer

Modo crashes instead of refusing a bad file (LXE-00). Write only the verified
layout, and test every exporter change against the oracles before shipping.
The export must be its own action with its own destination; it does not
replace Operandi's native save.
