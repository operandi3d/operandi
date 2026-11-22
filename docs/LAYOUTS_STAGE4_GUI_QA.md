# Layouts stage 4: workbench edit mode, GUI check

Spec: `specifications/layouts-2026-11-16/BEHAVIOR-LAYOUTS-STAGE4.md`
(LAY-82, LAY-86, LAY-87, UIC-212). Use the QA app from
`scripts/bundle-dev.sh` with its own data folder (`OPERANDI_DEV_DATA`),
never your installed app. Tick each line; note anything odd next to it.
Start with a cube in Polygon mode.

## Nothing else changed

1. [ ] Open each Model toolbox tab (Create ... Polygon) and look at the
       top bar: exactly as in the main QA build (no new tab yet).
2. [ ] Layout > **Workbench**: a Workbench tab appears at the bottom of
       the strip, open, with an **Edit Workbench** button and "An empty
       workbench: Edit Workbench, then Pick to Add." Switch tabs: it
       stays in the strip.

## Edit in place

3. [ ] Click Edit Workbench: it becomes **Done**, and Pick to Add, Add
       Divider, Add Divider with Label, Add Tool Pipe, Undo Edit and
       Revert appear under it.
4. [ ] Click **Pick to Add**, go to the Polygon tab and hover buttons:
       each one lights green under the pointer. Click Bevel: Bevel does
       *not* start; the status line says it was added.
5. [ ] Still picking: start Bevel from its key (B), then click a row in
       Tool Properties (e.g. Inset): the value doesn't change, the row is
       added. Open the Command History (History tab), click a line: added.
       Click a Preset Browser entry: added. Press **Esc**: picking stops.
6. [ ] Back on the Workbench tab (still editing): the items are listed one
       per row. **Drag** one to another place: the order changes and stays.
7. [ ] Click **Add Divider with Label**: a "Section N" header appears
       below. Drag it between two items. Click its header: it does
       not fold while editing.

## Right-click properties

8. [ ] Right-click an item: Label, Tooltip, Command, Popup Face,
       Alignment, Proficiency, Toggle Style, Custom Icon..., Duplicate,
       Delete, and a greyed **Show in Form Editor** (hover: stage 7).
9. [ ] Right-click the Bevel item: the **Label** field already says
       "Bevel". Click into it and let go: the menu stays open and the
       field has the cursor. Type a new label, press Enter: the menu
       closes and the row shows it. Right-click again, click into
       **Tooltip**, type, then click outside the menu: the menu closes and
       the tooltip is set (hover the button in use later: it shows). Click
       into Label, type, press **Esc**: the menu closes, nothing changed.
       **Reset Label**: the name is "Bevel" again.
10. [ ] **Alignment** and icon-only: set two buttons to Normal and one to
       Wide. Click Done: the Normal ones share a row (a third each), Wide
       takes two thirds, Full Width the whole row. Edit again; on six tool
       buttons set Normal, clear the Label (empty, Enter): each row chip
       says "Icon only". Done: all six fit one row as icons; hover one:
       the tooltip names it. A command with no icon (e.g. Select All)
       with its Label cleared still shows its name.
11. [ ] **Toggle Style ▸ Checkbox** on the Bevel button: it shows as a
       checkbox, ticked while Bevel is open; clicking it starts Bevel.
       Toggle Button: a framed button lit while Bevel is open.
12. [ ] **Custom Icon...**: choose any PNG: the button wears it. **Clear
       Custom Icon** removes it.
13. [ ] **Duplicate** then **Delete** an item. **Undo Edit** takes each
       back. Cmd+Z still undoes the scene only.

## Using it

14. [ ] Click Done. Fold the "Section N" header: the items under it hide;
       unfold: they return. Click a workbench button: it runs as its
       toolbox button does (Bevel starts and is lit).
15. [ ] The picked Tool Properties row shows the live field while Bevel is
       open (change the value there: Tool Properties follows), and its name
       greyed after you drop Bevel.
16. [ ] Set action centre Selection, falloff Linear, Symmetry X and start
       Bevel. Edit Workbench ▸ Add Tool Pipe ▸ **As Preset**: a "Bevel
       Preset" button appears (As Macro and As Script are greyed, "not in
       Operandi yet"). Drop Bevel, reset the three to Automatic / none /
       off, then click Bevel Preset: all three come back and Bevel starts.

## The top row, saving and Revert

17. [ ] Layout > **Top Workbench**: a slim empty row with **Edit**
       appears under the menu bar. Edit it and Pick to Add a toolbox
       button: it lands on the top row, not in the Workbench tab. Layout >
       Top Workbench again: the row hides.
18. [ ] Quit and reopen the QA app: both workbenches are as you left them.
19. [ ] Edit Workbench ▸ **Revert**: the Workbench tab is empty again
       (shipped). Undo Edit then says there is nothing to undo.
