# LXO legacy import: open questions

- **LEG-Q01 — Answered (v2)**: import SUBD/PSUB as plain polygon cages for
  now (Jason, 2026-11-03); see LEG-06. A subdivision spec follows.
- **LEG-Q02**: Formats 2.1, 3.0 and 3.1 (four samples): layer records are
  shorter (58 and 66 bytes after the name) and ENCO is absent. Not studied.
- **LEG-Q03**: Meaning of the LAYR "rest" bytes; only the observed forms are
  admitted.
- **LEG-Q04**: The one ENCO 1 (ANSI) file: decode as which code page?
- **LEG-Q05**: Sculpt/multiresolution data inside LAYR; currently refused.
- **LEG-Q06**: Mesh instances (`meshInst`, 21 files) and procedural meshes
  that store no geometry (several "0 points" meshes in LOCAL-CORPUS.md).
- **LEG-IMPL-Q01 — Answered (v3)**: see LEG-13.
- **LEG-IMPL-Q02 — Answered (v3)**: see LEG-14.
