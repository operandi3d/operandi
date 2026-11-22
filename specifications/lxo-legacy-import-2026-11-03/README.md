# LXO import: Modo 8–10 era files (format 4.0 and 4.1)

**Status: v3 (partial).** v2: SUBD/PSUB import as plain polygon cages (LEG-06). v3: answers LEG-IMPL-Q01/Q02 (LEG-13, LEG-14). An extension of the import spec
`lxo-import-phase1-2026-11-05` (clean re-derivation; replaces the removed phase-1 spec). Observed on 2026-11-03 from the
structure of the Modo 8–10 sample scenes, from Modo 17.1v1 opening them and
reading every mesh back (six decimals), and from original test files written
in the old layout. REA-assisted: no. The sample scenes themselves are not
included and must not be.

## Covered

- `BEHAVIOR-LEGACY.md`: what differs from the admitted profile (versions,
  optional APPV, layer-record variants, 11-bit polygon corner counts,
  unflagged channel types, action-layer precedence for transforms, cache
  chunks to skip) and which files to refuse.
- `ORACLES.json` with `fixtures/`: four original files in the old layouts
  (Operandi-made, free to keep in the repo) and Modo's exact readback.

## Not covered (QUESTIONS.md)

Formats 2.x and 3.x, curves, the smooth subdivision surface, sculpt layers, ANSI
encoding, instances and procedural meshes.
