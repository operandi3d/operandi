# Pie engine: GUI checklist

Branch `pie-engine` (UIC-308..313). One pass in the QA app, with its own
data folder. Tick each line; note anything that looks or feels different
from the pies before this branch.

## Shipped pies look and work as before

- [ ] Control+Space over a view: View Direction opens at the pointer.
      Same eight buttons, same tight layout; the centre dot is now the
      name "View Direction" on a small plate (UIC-313).
- [ ] Hold, point up, release: Top view. Point at a diagonal button
      past a side's wedge: the button under the pointer wins.
- [ ] Tap Control+Space: the pie stays open; click Front: Front view.
      Tap again, click away from every slot: it closes, nothing runs.
- [ ] Esc closes an open pie.
- [ ] Alt+Q (Selection & Transforms), Control+2 (Shading Style),
      Control+1 (Viewport Toggles), Control+` (Layouts): each opens and
      runs a slot on release. Toggles show their box; the current view,
      shading style, mode and action centre show a filled dot.
- [ ] Modeling pie (bind it first, see below) in Edge mode: Bevel is
      greyed; pointing at it shows why under the pie; release runs nothing.
- [ ] An old input map with a "Pie: View Direction" key: the key still
      opens the pie after launch (migration), and the Input Map row reads
      "Pie: View Direction".

## Form Editor (System > Form Editor)

- [ ] The tree has a Pies group with the seven shipped pies.
- [ ] Select pie.view: a ring with eight slots laid out like the pie,
      "View Direction" in the middle.
- [ ] Click the bottom slot: its editor shows Holds Command, Command
      "View Bottom", Label "Bottom".
- [ ] Drag Top onto Bottom: they swap; the others stay. The tree marks
      the pie changed (•). Open the pie with its key: the swap shows.
- [ ] Undo Edit takes the swap back; Revert brings the shipped pie back.
- [ ] New Pie with id my.pie: eight empty slots. Set slot 1 to a command
      from History ▾, give it a label and an icon from Browse Icons;
      Icons: both, icon, text change the ring and the pie.
- [ ] Type a line no command has: the slot is red in the ring and in the
      pie, and does nothing.
- [ ] Holds Sub-pie: only pies that don't lead back are listed. Pick
      pie.shading.

## Pick and icons (UIC-314)

- [ ] my.pie slot editor: Pick sits next to History under Command. Click
      Pick: the pointer is the system pointing hand. Click the toolbox's
      Bevel button: Bevel doesn't run; the slot gets its line and icon,
      and its label (the slot had none). Undo Edit takes it back.
- [ ] Pick again, then Esc: picking stops, nothing changes. Pick, then
      right-click: the same.
- [ ] Workbench Pick to Add also shows the pointing hand.
- [ ] Select a toolbox form's button (e.g. toolbox.edge): Pick and
      History sit under its Command too, and the icon picker replaces the
      old Icon / Icon File text fields.
- [ ] Icon swatch opens Browse Icons: type "bev" narrows the grid; click
      an icon: the swatch and the slot (pie and ring) wear it; its name
      shows under the swatch.
- [ ] Choose File… a PNG, then an SVG: the slot wears each; the path
      under the swatch is in the data folder's Layouts/Icons. Delete the
      original file: the slot still shows it.
- [ ] Delete the copy in Layouts/Icons and reopen: the slot shows a
      dashed "?" box, the editor says "Missing file", nothing breaks.
- [ ] Clear: no icon (the label shows).
- [ ] Export Pie with a custom icon, import it on another data folder
      (or after deleting Layouts/Icons): the icon comes back.

## Sub-pies (UIC-310)

- [ ] Open my.pie (Open Pie button, or its key), point at the sub-pie
      slot (it has a small triangle at its outer end), release: Shading
      Style opens in the same place, its own name in the centre.
- [ ] Click a slot in it: it runs and everything closes.
- [ ] Into the sub-pie again; Esc goes back to my.pie; Esc again closes.
- [ ] Into the sub-pie again; click in the centre: back to my.pie.

## Keys (UIC-311)

- [ ] my.pie > Assign Key…: the Input Map opens on "Pie: my.pie" (or its
      label), waiting. Press W: the clash banner names Move. Undo, then
      press Control+Alt+P; Save Input Map. Control+Alt+P opens my.pie
      held; hold, point, release runs the slot.
- [ ] Input Map > Keys for: this layout; give a pie a key there: it works
      in that layout only.

## Export / Import (UIC-312)

- [ ] my.pie > Export Pie…: the save panel suggests my.pie.oppie; save.
- [ ] Import Pie… that file: asks Rename / Replace / Cancel (the ids are
      taken). Rename: my.pie.2 and pie.shading.2 appear; my.pie.2's
      sub-pie slot opens pie.shading.2. Replace: my.pie is overwritten.
      Cancel: nothing changes.
