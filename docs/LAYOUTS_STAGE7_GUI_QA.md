# Layouts stage 7: menus as forms and the Form Editor, GUI check

Spec: `specifications/layouts-2026-11-16/BEHAVIOR-LAYOUTS-STAGE7.md`
(LAY-83, LAY-87, LAY-Q53, UIC-10, UIC-210, UIC-213..215). Use the QA app
from `scripts/bundle-dev.sh` with its own data folder
(`OPERANDI_DEV_DATA`), never your installed app. Tick each line; note
anything odd next to it. Start with a cube on the Model layout.

## Menus look the same

1. [ ] Texture, Render and Help look exactly as in the stage 6 build: same
       entries, same keys after the names, same greyed entries and
       tooltips (Help > Documentation "Coming soon"; without Cycles,
       Render and Preview greyed). Each still works (Texture > Open UV
       Editor toggles; Render > Open Render Window opens it).
2. [ ] A new **System** menu (between Window and Help) holds **Form
       Editor**.

## The Form Editor

3. [ ] System > Form Editor: a floating window opens (about 820 x 600);
       the menu item is ticked. Choose it again: it closes.
4. [ ] Left side: groups Toolbox, Toolbars, Menus. Open Menus: menu.help,
       menu.render, menu.system, menu.texture. Type "report" in the
       search: only menu.help is left. Clear it.
5. [ ] Click **menu.help**: it opens to show Documentation and Report
       Bug, and the right side shows the form's properties.
6. [ ] Click **Report Bug**; change its Label to "Tell Us" and press
       Enter. menu.help gets a "•" (changed). Help menu now says "Tell
       Us".
7. [ ] Drag "Tell Us" above Documentation in the tree: the Help menu
       order follows.
8. [ ] Select menu.help, click **Revert**: the Help menu is back as
       shipped and the "•" is gone.

## Building a form

9. [ ] Type "Mine" and click **New Group**; type "my.tools" and click
       **New Form**. In its properties set Group to Mine.
10. [ ] In the main window run Cube once. Back in the Form Editor, click
        **History ▾**, pick the Cube line, click **Add Command**. Add a
        Divider; pick toolbox.uv.ops from the Sub-form list and click
        **Add Sub-form**.
11. [ ] Click **Preview Form**: the form draws under the properties with
        Cube, a line and the UV ops buttons. Click Preview Form again to
        hide it.
12. [ ] Click **Save Form**: the status line names a file in the data
        folder's Saved Forms folder.
13. [ ] Tick **Customizable Workbench** on my.tools; type
        `operandi.workbench.edit form:my.tools` in the command line: it
        starts editing (no "isn't a workbench" error).
14. [ ] Set **Slot In Form** to `toolbox.create`, Slot At **head**: the
        Create tab now starts with my.tools' buttons. Clear the slot: they
        go.

## Find Form, Show in Form Editor

15. [ ] Click **Find Form**, then click the Cube button in the Create
        tab: Cube doesn't start; the Form Editor shows toolbox.create with
        the row holding Cube selected. Find Form again then Esc: it stops.
16. [ ] Open the Workbench tab, Edit Workbench, add a button, right-click
        it: **Show in Form Editor** is no longer greyed and opens the Form
        Editor on that item.

## Restart

17. [ ] Quit and relaunch: my.tools and the Mine group are still there,
        and the Form Editor reopens where it was if it was open.
