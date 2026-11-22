# Modo content folder: GUI check

Build: the QA app named in the thread. Use your own Modo content folder
(any folder with .lxo / .lxl / .lxp files works). Note the folder's
modified times first (Finder ▸ View ▸ Show View Options ▸ Date Modified).

1. **MC-T1 Add as Location.** Open the Preset Browser (F6). Drag the
   content folder from the Finder onto the browser. A small menu shows
   Add as Location / Copy Into Library / Cancel. Pick Add as Location: it
   appears as a root within a few seconds and lists. Nothing in the folder
   changed (dates the same, no new files).
2. **Kind filter.** Use Show ▾ beside the F toggle: Meshes shows .lxo,
   .lxl and model files; Scenes shows .lxo; an empty folder under Meshes
   says "Add a folder of models to see them here." Type
   `operandi.pbview.kindFilter kind:materials` once in the command line.
3. **MC-T2 greyed presets.** Gear ▸ untick Show only usable files. A
   material .lxp shows greyed, with Modo's own thumbnail if it has one, and
   kind "Modo material preset". Double-click: the status line says
   "Modo material preset. Not supported yet." and the scene is unchanged.
   Right-click shows only Show in Finder.
4. **MC-T4.** .py and .lxm files never show, with the option on or off.
5. **MC-T3 .lxo into the view.** Drag a .lxo tile ("geometry only" on the
   tile) into a 3D view: its meshes are added (Shift: under the pointer); its
   materials appear as named groups in the Shader Tree. Cmd+Z removes them
   in one step. Double-click instead opens it as its own scene.
6. **Stage 2 .lxl.** Double-click a .lxl mesh preset: a new mesh item at
   the origin, selected. Drag one into a view: it lands at the origin;
   hold Shift as you let go and it lands on the surface or work plane under
   the pointer (as for model files, PG-06). Cmd+Z removes it in one step. A preset Operandi can't
   read says why in the status line and adds nothing.
7. **MC-T5.** In the Finder, copy (or symlink) a folder into
   `~/Library/Application Support/Operandi/Library/`. It appears as a root
   without a restart.
8. **Copy Into Library.** Drop a small folder on the browser ▸ Copy Into
   Library. The status line says when it's done and the copy appears as a
   root. Do it again: it asks before copying over.
