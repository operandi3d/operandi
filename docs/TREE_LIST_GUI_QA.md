# Shared tree list, panel tabs and viewport header: GUI checks

For Jason or Codex (Claude can't click Operandi). Use the QA build named in
the report, on a throwaway scene with a few meshes, a camera and a light.
Each line: do this → expect that. Rule numbers are the UI bible's.

## Items (UIC-150)
1. The Items tab: Add Item ⏷ at the left; Select, Filter, Sel, Notes
   (greyed), then F at the far right. Header row: eye, E, Name; thin column
   lines run to the bottom of the panel even below the last row.
2. Rows are compact (about 17 px); children hang off dotted lines with a
   small dot where they meet. The current scene row is bold, with "*" after
   a change; a hidden item's name is dim; an empty new mesh's name is grey.
3. Click a row → only it selected (darker row, orange name). Ctrl+click
   (the real Ctrl key on a Mac; Cmd does not) adds/removes. Shift+click →
   the range from the last clicked row; the first one stays a shade
   lighter (primary). Ctrl+click never opens a right-click menu.
4. Click a selected row, wait a second, click again → its name becomes a
   field; Enter renames, Esc cancels, clicking elsewhere keeps the text.
   A quick double-click on a parent selects it and all its children
   instead.
5. Drag down the eye column across three rows → all three hide; one Cmd+Z
   shows them again. Alt+click one eye → only that item shows; Alt+click it
   again → the others come back.
6. Drag a row onto the middle of another → it becomes a child (outlined
   target). Drag it onto the top quarter of a row → an insert line above
   that row, indented to its level, with a small tick at its start; drop →
   it moves there without becoming a child (Edit menu: Undo Reorder; from
   another parent: Undo Parent / Undo Unparent, world position kept). The
   bottom quarter of a row → line below it; on an open parent, the line is
   indented one level deeper and the item becomes its first child. Below
   the last row → last top-level item. Select two rows (Ctrl+click, the
   lower one first) and drag one → both move, in list order. Drag a parent
   over its own children → no line, no outline, nothing happens on
   release. Save, reopen → the order is kept. Drag a row onto another open
   scene's row → moved there (Option: copied).
7. Click F → a search row (field, Text/Pattern, Aa, Flat). Type part of a
   name → matches and their parents (dim). Pattern `m*` → whole-name
   matches. Flat → matches only, no tree. Click F again → full list back,
   selection kept. Relaunch → the search is as you left it.
8. Drag the divider between E and Name → the column widens; narrow the
   panel a lot → the list scrolls sideways.

## Shader Tree (UIC-152) and Groups (UIC-153)
9. Shader Tree tab: row 1 "View [Shader Tree ⏷]" and Assign Material…;
   row 2 "Filter [(none) ⏷]", Add Layer ⏷, F at the far right. Columns:
   eye, Name, B, %, Effect. Render is bold.
10. A texture with a locator shows a small +; click it → the locator row
    under it; − hides it. Blend (B) and Effect still open their menus;
    dragging % still changes opacity in one undo step.
11. Drag a layer onto a Group's middle → inside it; between two layers →
    moved there. Shift+click a triangle → the whole branch opens/closes.
12. Groups tab: New Group at the left, F at the right; eye, Name. Make a
    group of two items → one row "Group"; its triangle shows the members;
    its eye hides/shows both in one step. Right-click the group → Rename…,
    Delete Group, Add Selected, Remove Selected. Empty Groups list → just
    the header and lines, no message.

## Panel tabs (UIC-155..157)
13. "+" after the tabs → Data Lists ▸ (Items, Shader Tree, Groups) and
    Properties ▸ (Properties); kinds already in the group are greyed. In the
    lower group's +, pick Groups → Groups moves to the lower group and comes
    to the front there.
14. Drag a tab sideways → orange insert line; it lands there. Right-click a
    tab → Float in Its Own Window, Close Tab (not on a group's only tab).
15. Narrow the right panel → tab names cut off with "…"; narrower still →
    a ▾ lists the hidden tabs. Relaunch → tabs, order and front tab kept.

## Viewport Properties header (UIC-103, UIC-111)
16. Open a view's gear. Header: Presets ⏷, One Tab ⏷, All Tabs ⏷.
17. One Tab ⏷ on Visibility → (new for 3D View) (type a name, Enter),
    (rename), (delete) greyed until a preset exists. Change Show Grid, pick
    the preset → only Visibility's rows change back.
18. All Tabs ⏷ → Find field, presets, (new, all tabs …), (new, selected
    tabs …) with a tick per tab, (rename), (delete), Load/Save Default for
    3D View (Load greyed until saved). Save Default, split the view → the
    new view starts with those settings.
19. With the pointer over the popover press O → all tabs in one column;
    O again → one tab. O over the view itself still opens/closes it.
20. Labels: on Drawing and Control the label column is narrow (View Type,
    Background hug their fields); on Active Meshes it is wider; every label
    in a tab ends on the same edge, no colons.
