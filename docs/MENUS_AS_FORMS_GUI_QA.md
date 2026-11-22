# Menus as forms, GUI check

Follows layouts stage 7 (LAY-Q66; UIC-10, UIC-210, UIC-213). Every menu is
now drawn from a form. Use the QA app from `scripts/bundle-dev.sh` with its
own data folder (`OPERANDI_DEV_DATA`), never your installed app. Tick each
line; note anything odd next to it. Start with a cube on the Model layout.

## The menus look and work as before

1. [ ] Open each menu, File to Help: same entries, same order, same
       separators and submenus, same keys after the names as the stage 7
       build. Cmd+S, Cmd+W and Cmd+Shift+W still show on Save, Close
       Scene and Close All.
2. [ ] Edit: Undo reads "Undo New Mesh" (or your last step) and Redo is
       greyed with a tooltip saying why. Undo once: Redo now names the
       step.
3. [ ] Select: the current mode is ticked (Polygons); pick Edges; it is
       ticked next time. Lasso Style > Rectangle ticks; Partial toggles.
4. [ ] View: with one view the entry reads **Four Views**; choose it,
       then the entry reads **Maximise Viewport** or **Restore Viewport
       Layout** as the views change. Tear Off View and Close View are
       greyed with one view and work with two.
5. [ ] Item > Transform: greyed in Polygon mode with "Select an item";
       in Item mode with the cube selected, Freeze Scale works.
6. [ ] Activate Move: Geometry > Polygon > Boolean..., Solid Drill...,
       Axis Drill..., Remesh... and Automatic Retopology... are greyed
       ("Drop or Cancel the active tool first"), as their toolbox
       buttons are. Drop the tool: they work again.
7. [ ] Window > Frame Time ticks and shows the frame time; Compact and
       Standard resize the window.

## Lists that change while the app runs

8. [ ] File > New, then File > Current lists both scenes with the
       current one ticked; pick the other one.
9. [ ] Layout > New Layout... opens the New Layout dialog; make "Test".
       Layout > Layouts now lists it, ticked. Layout > Rename Layout... and
       Delete Layout... work on it (both greyed on Model).
10. [ ] Layout > Windows/Palettes: open Groups; it is ticked; Toggle
        Palettes hides it.

## Every click is a command

11. [ ] Open the Command History palette. Choose Select > Grow, View >
        Frame All and Window > Frame Time: each shows as a line (marked as
        from a menu).
12. [ ] Geometry > Mesh Cleanup... opens its dialog; Cancel.

## Menus in the Form Editor

13. [ ] System > Form Editor > Menus group: menu.bar, menu.file,
        menu.file.export, ..., menu.window are listed.
14. [ ] Open menu.file.recentProjects: one item, "Recent Projects
        (live)". It has no controls to expand; Move Up/Down and Delete
        work on it as one item (then Revert the form).
15. [ ] Select menu.help, set its Label to "Assist": the Help menu is
        renamed "Assist" in the menu bar. Revert puts "Help" back.
16. [ ] Open menu.file and select Save: its Key Equivalent property
        shows "s" (the Cmd+S on the Mac menu bar).

## Windows / Linux only (skip on macOS)

17. [ ] The in-window menu bar shows all thirteen menus with the same
        submenus as the Mac bar; Help starts with About Operandi.
18. [ ] Window has no "Float ..." checkboxes; Layout > Windows/Palettes
        opens Toolbox, Items and Properties as palettes.
19. [ ] Clicking an in-window entry runs it (System > Form Editor opens).
