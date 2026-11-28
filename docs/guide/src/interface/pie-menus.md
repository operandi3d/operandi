<!-- help-ids: operandi.pie.open, operandi.pie.assignKey, operandi.pie.export, operandi.pie.import, operandi.form.slot, operandi.form.swap, operandi.form.pick, operandi.form.setIcon, operandi.form.chooseIcon ; template: concept ; since: 0.1.0 -->
# Pie menus

A pie menu is a ring of up to eight commands that opens around the
pointer when you press its key. Hold the key, point toward a slot and let
go: that slot runs. You can change any pie, make your own, put pies inside
pies and give any of them a key.

## Using a pie

1. Point where you want the pie, then press and hold its key (for
   example **Control+Space** for **View Direction**).
2. Move the pointer toward the slot you want. The slot lights up, and a
   small arrow next to the pie's name points at it.
3. Let go of the key. The slot runs, as its button or shortcut would.

Tap the key instead of holding it and the pie stays open: click a slot to
run it, or click anywhere else (or press **Esc**) to close it.

- The pie's name sits in the centre, with its slots around it.
- A greyed slot can't run right now; point at it to see why under the pie.
- A slot in red runs a command Operandi doesn't know, so it does nothing.
- Slots with a box or a dot show a state: a ticked box is a setting
  that is on, a filled dot the choice in use (the current view, shading
  style or selection mode).

## Pies inside pies

A slot can open another pie, called a sub-pie. It has a small triangle at
its outer end.

1. Point at the slot and let go of the key (or click it). The sub-pie
   opens in the same place, with its own name in the centre.
2. Click a slot to run it.
3. To go back to the pie you came from, press **Esc** or click in the
   centre. In the first pie, **Esc** closes it.

A pie can never hold itself, not even through one of its sub-pies.

## The pies Operandi starts with

| Pie | Key |
|---|---|
| View Direction | **Control+Space** |
| Selection & Transforms | **Alt+Q** |
| Shading Style | **Control+2** |
| Viewport Toggles | **Control+1** |
| Falloffs | none |
| Action Centers | none |
| Modeling | none |
| Panels (pops out the Graph Editor, Items, Properties, Timeline or Toolbox) | **Control+3** |
| Layouts (one slot per layout) | **Control+` (backtick)** |

## Editing a pie

Pies are edited in the **Form Editor** (**System > Form Editor**), under
**Pies**.

1. Click a pie in the tree. On the right, a ring shows its eight slots as
   the pie lays them out, with its name in the middle.
2. Click a slot. Below the ring, choose what it **Holds**:
   - **Command**: type a command line in **Command** and press Enter,
     pick one you ran recently from **History**, or use **Pick** (below).
   - **Sub-pie**: choose the pie it opens. Only pies that don't lead back
     to this one are listed.
   - **Panel popover**: choose a panel in **Panel**. The slot pops that
     panel out on its own next to the pointer, as the **Panels** pie does.
   - **Empty**: the slot stays blank and keeps its place.
3. Give the slot a **Label**, and an **Icon** if you like (below).
4. To move a slot, drag it onto another slot: the two swap places and the
   rest stay where they are.

### Taking a command from a button

1. Click **Pick** next to **Command**. The pointer turns into a pointing
   hand.
2. Click any button in the toolbox, a toolbar or a menu in the window.
   It doesn't run: its command goes into the slot, with its icon, and its
   label if the slot has none yet.

Press **Esc** or right-click to stop picking without choosing. Every
button you select in the Form Editor has the same **Pick** next to its
**Command**, and the same icon choices below.

### Choosing an icon

- Click the square icon button next to **Icon** to **Browse Icons**: type
  part of a name to narrow the grid, then click an icon.
- **Choose File…** uses a picture of your own (PNG, JPEG or SVG).
  Operandi keeps a copy in its data folder, so moving or deleting the
  original doesn't matter.
- **Clear** takes the icon off.

The icon's name, or the copied file, shows under the button. If an icon
file has gone missing, the slot shows a box with a question mark instead.

**Icons** in the pie's properties chooses whether slots show their label,
their icon, or both. A slot without an icon always shows its label.

Your changes are kept as your own copy of the pie. **Undo Edit** takes the
last change back, and **Revert** puts a pie Operandi came with back the
way it was. On a pie you made, the same button deletes it.

### Making a new pie

1. Type an id for it in the field next to **New Form** (for example
   `my.views`).
2. Click **New Pie**. It starts with eight empty slots.
3. Fill the slots as above, and set **Label** to the name the centre
   shows.

## Giving a pie a key

Every pie, yours too, has a row in the **Input Map** (**Edit > Input
Map…**) named **Pie:** and its name.

1. In the Form Editor, select the pie and click **Assign Key…**. The Input
   Map opens on the pie's row, waiting.
2. Press the keys you want. If another action already uses them, the
   Input Map says which, and you can take the key from it or undo.
3. Click **Save Input Map**.

A key set while **Keys for** shows a layout works in that layout only.

## Sharing a pie

**Export Pie…** saves the selected pie to one file together with every
sub-pie it opens, and any icon pictures of your own that they use. **Import Pie…** (next to **New Pie**) reads such a file.

If a pie in the file has the same id as one you already have, Operandi
asks first:

- **Rename** keeps yours and gives the imported ones new ids (such as
  `my.views.2`).
- **Replace** puts the imported ones over yours.
- **Cancel** imports nothing.

## Related

- [System menu](../menus/system.md)
- [Keyboard shortcuts](../reference/keyboard-shortcuts.md)
