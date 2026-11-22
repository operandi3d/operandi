# Operandi Library pack (F1): GUI check

QA app: `~/Developer/Operandi-Builds/Operandi library-pack <commit>.app`
(built with Cycles, so thumbnails render). The pack is installed at
`~/Library/Application Support/Operandi/Library/Operandi Library`
(unzipped from `~/Developer/Operandi-Content/Operandi-Library-2026.11.0.zip`).
To see the empty state, quit and move that folder out of `Library/` for a
moment (step 5), then put it back.

1. **Basics on a fresh browser (F1 item 4).** Window ▸ Preset Browser.
   Expect a bold "Basics" root at the bottom of the tree, just above
   "(add path…)", with no × in its row. Click it: six materials (Clay,
   Emissive, Glass, Metal, Plastic, Rubber). Within a few seconds each shows
   a rendered cutaway ball. Hover one: the tooltip ends
   "Licence: GPL-3.0-or-later · Operandi Basics (read-only)".
2. **Basics are read-only (UIC-117).** Right-click a Basics material:
   Rename…, Duplicate… and Delete… are greyed; hovering says "Basics are
   built into Operandi and read-only". Load still works (select a material
   group in the Shader Tree, double-click Plastic: one Cmd+Z undoes it).
   Right-click the Basics root: New Folder…, Save … Here…, Rename… and
   Remove… are greyed with the same note; Show in Finder and Refresh work.
3. **The pack shows as a root (F1 item 3).** The tree has a bold root
   "Operandi Library 2026.11.0". Hover it: name and version, "32 items
   (32 CC0-1.0)" and its folder. Open it ▸ Starter ▸ Bricks113A.
4. **Licence, credit and source per entry.** Hover the Bricks113A preset (and
   its texture set entry): the tooltip shows "Licence: CC0-1.0", "Credit:
   ambientCG", "Source: https://ambientcg.com/a/Bricks113A", "Licence
   checked 2026-11-11" and the pack title. Same in list view (≡). Check one
   Poly Haven item (e.g. red_brick) says Poly Haven.
5. **Empty state (no pack).** Quit, move `Library/Operandi Library` to the
   Desktop, relaunch, open the browser. Above the contents: "More
   materials:" with "Get the free Operandi Library…" and "Locate Library
   Folder…". The pack's root shows "(not found)", greyed.
   - Click "Get the free Operandi Library…": your web browser opens
     https://operandi3d.org/library. Operandi downloads nothing.
   - Click "Locate Library Folder…": a folder chooser. Pick the folder on
     the Desktop: it's added as "Operandi Library 2026.11.0" and shown; the
     offer goes away.
   - Remove that root (× on its row, confirm), move the folder back into
     `Library/`: within two seconds nothing reappears (a removed pack stays
     removed). Use Locate… on `Library/` to bring it back.
6. **Two versions (F1 item 3).** Copy the pack folder in `Library/` to
   "Operandi Library 2026.12.0" and in the copy's manifest.json change
   "version" to "2026.12.0". Within two seconds a second root appears; the
   old one reads "Operandi Library 2026.11.0 (newer: 2026.12.0)" and its
   tooltip says where the newer one is. Delete the copy afterwards.
7. **Commands.** In the command line: `operandi.pbview.libraryGet` opens the
   page in the web browser; `operandi.pbview.libraryGet url:https://example.com`
   opens that address instead. `pbview.basePathAdd path:"<Library folder>"`
   adds the pack, titled by its manifest, like Locate… does.
8. **Matcaps and environments (2026.11.4).** Install
   `Operandi-Library-2026.11.4.zip` the same way. The pack root shows
   "311 items (311 CC0-1.0)". Open Matcaps ▸ Colour: round tiles; hover one:
   "Licence: CC0-1.0", "Credit: hecko", the OpenGameArt source. Clay ▸ has
   no Clay Studio or Basic Grey (they're built into the Matcap menu). Open
   Environments ▸ Studio: three HDR thumbnails; hover Studio Small 08:
   "Credit: Sergej Majboroda (Poly Haven)", source polyhaven.com. Double-click
   it: the 3D view's environment shows the studio; Cmd+Z puts the old one
   back. Do the same with Night ▸ Night Sky 003 (an .exr from ambientCG).
9. **Meshes and indoor environments (2026.11.5).** Install
   `Operandi-Library-2026.11.5.zip`. Open Meshes ▸ Furniture: Table and
   Stool with clay thumbnails; hover one: "Licence: CC0-1.0", "Credit:
   Operandi contributors". Double-click Table: a 1.2 m table stands on the
   ground at the origin; Cmd+Z removes it. Shift-drop Mug onto the table
   top: it rests there. Open Environments ▸ Indoor: 11 HDR thumbnails;
   double-click Cayley Interior and check the room shows in the 3D view.
