# Layouts stage 5: per-layout key maps, GUI check

Spec: `specifications/layouts-2026-11-16/BEHAVIOR-LAYOUTS-STAGE5.md`
(LAY-20, LAY-71, UIC-201). Use the QA app from `scripts/bundle-dev.sh`
with its own data folder (`OPERANDI_DEV_DATA`), never your installed app.
Tick each line; note anything odd next to it. Start with a cube in Polygon
mode, on the Model layout.

## Nothing else changed

1. [ ] Model and UV look and behave as in the main QA build: same
       toolbox, same tooltips (hover Move: `[W]`), W/E/R start
       Move/Rotate/Scale in both layouts.
2. [ ] Edit > Input Map…: a new **Keys for: Global** choice at the top;
       every row as before, no ◆ or ◇ marks anywhere.

## Set a key for one layout

3. [ ] Choose **Keys for: This layout (Model)**. The hint says keys set
       here work only in Model. Tick **Only this layout's keys**: no rows.
       Untick it.
4. [ ] Search "Move", click its shortcut (W), press **R**. No red clash;
       the status line says R runs Move in Model instead of Scale. Move's
       name turns orange and gets a cyan **◆**.
5. [ ] Search "Scale": its key shows "—" and a cyan **◇**; hover the ◇:
       it says that in Model this key runs Move.
6. [ ] Switch **Keys for** back to Global: Move shows W with a ◆ (hover:
       "The Model layout sets this key: R there (global key: W)"), Scale
       shows R with a ◇. Click **Save Input Map**.
7. [ ] In the viewport press **R**: Move starts (not Scale). Hover the
       toolbox Move button: the tooltip shows `[R]`. Drop the tool.

## Switching swaps the key

8. [ ] Click the **UV** tab. Press **R** over the 3D view: Scale starts.
       Hover Move: `[W]`. Drop the tool.
9. [ ] Press **Ctrl+Shift+Tab**, keep Ctrl+Shift held, Tab to Model,
       release. Model shows. Now press plain **R**: Move starts (nothing
       acts as if Ctrl or Shift were still down). Drop the tool.
10. [ ] Start Bevel (B) and, with it still active, click the UV tab: the
        layout doesn't switch and the status says to drop the tool first.
        Drop it.

## Save, reopen, Revert, Clear

11. [ ] Quit and reopen the QA app: Model is open and **R** still starts
        Move; in UV R is still Scale.
12. [ ] In Model split the view (any way), then Layout > **Reset Layout**.
        The question has an unticked "Also clear its 1 layout key" box.
        Click Reset: one view again, and the status line says the key was
        kept. **R** still starts Move.
13. [ ] Layout > Reset Layout again, tick the box, Reset: R is Scale
        again in Model.
14. [ ] Set Move to R for Model again (steps 3–4) and Save. Open the Input
        Map, click **Clear this layout's keys**, Save: R is Scale in
        Model, no ◆ marks.
15. [ ] Layout > New, tick "Copy the current layout", after setting a key
        for Model first: the copy keeps it (R = Move). Layout > New
        without copy: R = Scale there.

## Scripts

16. [ ] Command line: `operandi.layout.setKey action:Move key:Ctrl+M`: the
        status says "Move: Ctrl+M in Model"; hover Move: `[Ctrl+M]`.
        `operandi.layout.clearKeys`: back to W.
