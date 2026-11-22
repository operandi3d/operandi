# Questions for Jason (builder uses the recommended answer until told otherwise)

- **Q1 Direct tool on a mesh with ops.** Recommended: refuse with a one-line
  message offering Edit Base Mesh or Freeze (SAFE-05). Alternative: quietly
  edit the Base Mesh.
- **Q2 A failing op.** Recommended: it passes its input through, row turns
  red with the reason, ops above still run (SAFE-04). Alternative: stop the
  whole stack at the failing op.
- **Q3 Enable Deformers (O).** Recommended: hides deformers only; MeshOps
  always show (MO-72).
- **Q4 Shortcuts.** Shift+S, Shift+F, backtick and Tab-over-list (MO-16,
  MO-24) may collide with Operandi's map. Builder lists collisions; Jason
  picks.
- **Q5 Roadmap.** Recommended: Booleans → MeshOps Stages 0–1 → MeshFusion
  (SCOPE.md).
- **Q6 Where a new op lands** (MO-18): above the selected row, provisional.
