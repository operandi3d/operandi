# Detriangulate preview: GUI check

For Jason or Codex. QA build from branch `ui-hints-detri`.

1. Make a mesh of triangles (e.g. a plane, Triple it). Polygon tab >
   Detriangulate...: while the dialog is open, the diagonals it will
   remove show in red in the viewport.
2. Change Max Angle to 10°: the red edges go away (nothing would join).
   Put it back: they return. Tick Keep Material Boundary on a mesh with two
   materials: edges across the boundary drop out of the preview.
3. In Polygon mode with some triangles selected, only edges among the
   selected ones show.
4. OK: the red edges are exactly the ones that disappeared, and the
   preview is gone. Undo, open again, Cancel: the preview goes, nothing
   changed.
5. Provisional: the preview draws over the mesh (hidden edges show too).
