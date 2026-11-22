# LXO export: open questions

- **LXE-Q01**: Do Modo 16.0v4 and 17.0v8 open the LXE layout? Only 17.1v1 was
  tested. v3's render item removed the version-related crash in 17.1v1
  (LXE-O-15), which makes older readers more likely to cope, but test them
  before claiming support.
- **LXE-Q02**: The APPV words are copied from Modo 17.1v1's own saves; their
  individual meanings are not established. Only (17, 4001, 1, any) is
  verified.
- **LXE-Q03**: Pivots and centre offsets (the import spec's pivot stack),
  rotation orders other than ZXY, shear from non-uniform scale under rotated
  parents.
- **LXE-Q04**: Empty meshes (LAYR with no PNTS/POLS), meshes with unused
  points, n-gons above 4 sides, and very large polygons (count limit 1023).
- **LXE-Q05**: SUBD/PSUB polygons, subdivision weights and creases, weight,
  colour, normal and morph maps, selection sets.
- **LXE-Q06**: Instances and shared geometry.
- **LXE-Q07**: Whether File > Import and drag-and-drop treat these files the
  same way as File > Open.
- **LXE-Q08**: LXL (preset) export.
- **LXE-Q09**: Undo history after opening (not relevant to a file, listed for
  completeness).
- **LXE-IMP-Q01 — Answered (v2):** the importer admits the LXE profile;
  the exporter stays minimal. See BEHAVIOR-IMPORT-PROFILE.md.
- **LXE-Q10**: admitting files saved by Modo 17.1v1 itself (full records,
  PAKG subchunks), including LXE files Modo has re-saved.
