<!-- help-ids: operandi.rig.schematicView, operandi.rig.workspaceAdd, operandi.rig.workspaceRename, operandi.rig.workspaceClear, operandi.rig.workspaceRemove, operandi.rig.workspaces, operandi.rig.schematicPlace, operandi.rig.schematicRemove, operandi.rig.schematicArrange, operandi.rig.schematicFit, operandi.rig.schematic ; template: panel ; since: 0.1.0 -->
# Schematic

The Schematic shows your rig as boxes and wires: items and rig pieces (constraints, links, modifiers) as nodes, and the channels they pass between them as wires. Open it with Animate > Schematic.

## Workspaces

A big rig is easier to read in pieces, so the Schematic shows one **workspace** at a time: a named page that holds only the nodes put on it, at the places you put them. Workspaces are saved with the scene.

- Pick a workspace from the list at the top left.
- **New Workspace…** adds an empty page and asks for its name.
- Right-click empty space for **Rename Workspace…**, **Clear Workspace** (takes every node off the page) and **Delete Workspace**. None of these change the scene: the items and pieces stay.

A scene with a rig but no workspace yet shows everything on one page called **Workspace**, laid out for you. It is saved as a real workspace the first time you move a node or change the page. When workspaces exist, rig pieces that are on none of them show on a page called **Unplaced**.

## Put nodes on a page

- **Add Selected** puts the selected items on the page. Select them first in a 3D view or the Items list; the button is greyed until you do.
- **Add Rig** puts the selected items' rig pieces on the page, with the items those pieces use.
- Drag a channel's label from the **Properties** panel onto the page: its item comes onto the page where you let go (or stays where it is), with that channel as a port on both sides, ready to wire. Drag one of several selected labels to bring them all.
- **Add ▾** adds a modifier (Expression, Blend, Math and so on) to the selected item and puts it on the page. Wire it by dragging from port to port.
- A new rig piece that touches an item on the page shows up beside that item.

Added nodes land in the middle of the view, laid out together, and a line at the bottom of the page says what was added. Nodes already on the page aren't added twice.

A port with a ring around it has a wire to a node that isn't on this page. Hover it to see which, and double-click it to bring that node onto the page beside this one.

## Arrange

**Arrange** lays the page out by data flow: what drives sits on the left, what is driven on the right, each modifier between its inputs and the item it drives, ordered to cross as few wires as it can. Nodes you moved by hand stay where you put them. Arrange is one undo step.

## Move around

The Schematic moves like an orthographic 3D view, with the same keys from your navigation preset (Preferences > Input), so your 3D-view habits carry over:

- The wheel or a pinch zooms about the pointer.
- The preset's pan and zoom drags work here. With the Operandi preset, Alt+Shift+drag pans and Alt+Ctrl+drag zooms. The orbit drag pans too, since a page can't turn.
- Dragging empty space or middle-dragging also pans.
- **Fit** frames every node on the page. Shift-click it to frame only the node you clicked last.

## Work with nodes and wires

- Drag a node to move it. Its new place is saved with the scene, and the move is one undo step.
- Click an item's node to select the item.
- An expression node shows its formula under its name; hover it to see the whole formula and what it drives.
- Drag from an output port (right side) to an input port (left side) to wire them. The **+** port on an item asks which channel to use.
- Click a wire to select it, then press Delete to delete the link.
- Press Delete with a node selected to take it off the page. The scene keeps it. To delete a rig piece from the scene, right-click it and choose **Delete Piece**.

## Scenes from other apps

Opening an LXO scene brings its schematic pages along: each workspace with its name and order, and each node at its place. Backdrops, notes and elbows aren't shown yet; the import report counts them.
