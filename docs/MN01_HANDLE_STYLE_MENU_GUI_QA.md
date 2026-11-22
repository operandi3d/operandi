# Handle style in the View menu (MN-01): GUI checklist (2026-11-19)

For Jason or Codex, on the QA build.

1. View > Tool Handles > Draw Style lists Invisible, Basic, Advanced,
   Default. Basic is ticked on a fresh data folder.
2. Pick Invisible, open Move (W) on a selected polygon: no handles draw.
   Drag in empty space away from the mesh: the polygon still moves
   (hauling). Invisible is now ticked.
3. Pick Default: handles come back, Basic is ticked, Default never is.
4. Pick Advanced, quit and relaunch: Advanced is still ticked, and
   Preferences > Tool Handles shows Advanced.
