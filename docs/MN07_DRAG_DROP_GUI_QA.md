# Drag and drop gaps (MN-07): GUI checklist (2026-11-19)

For Jason or Codex, on the QA build.

1. Drag a PNG from Finder onto a cube in a 3D view (pointer on the
   surface): no Backdrop; the Shader Tree has a new Image Map layer with
   that picture at the top of the cube's material group and the cube shows
   it. One Undo removes it. An .hdr / .exr on the cube does the same.
2. Drop a PNG on empty space in a 3D view: a Backdrop, as before. An .hdr
   on empty space: the environment image, as before.
3. Drop two pictures and two .opscene files at once on a mesh: both
   pictures become layers; both scenes open, each as its own scene.
4. Item mode, select a mesh, open Groups and make a group. Drag the
   selected mesh from the 3D view onto the group's row: it joins the group
   (one Undo). Dragging an unselected item still lassos as before.
