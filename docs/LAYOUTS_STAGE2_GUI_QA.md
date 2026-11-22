# Layouts stage 2: GUI check

Spec: `specifications/layouts-2026-11-16/BEHAVIOR-LAYOUTS-STAGE2.md`.
Use the QA app from `scripts/bundle-dev.sh` with its own data folder
(`OPERANDI_DEV_DATA`), never your installed app. Tick each line; note
anything odd next to it.

## Looks the same at rest

- [ ] Open a saved Model layout from stage 1. Check that its panels,
      sizes and tabs are preserved. Each view header has a small ▾ at
      its far right.
- [ ] Help is the last menu: Documentation is greyed with the tooltip
      "Coming soon"; Report Bug opens a filled-in GitHub issue in the
      browser (nothing is sent until you submit it).
- [ ] View menu no longer has a Layout submenu; Layout menu has the locks.

## Layout menu (UIC-199, LAY-42..48)

- [ ] Layout > New Layout…: a dialog with Name and "As Copy of Current
      Layout" (unticked). Make "Blank": it appears in the Layouts menu
      and is current. The view area is one empty panel with a corner dot
      (top left) and ▾ (top right). Toolbox and right columns stay.
- [ ] New Layout with the box ticked: a copy of the current arrangement.
- [ ] Save Layout As…: the current arrangement under a new name.
- [ ] Rename Layout… on a layout you made renames its menu entry; on Model/UV it
      is greyed with a reason.
- [ ] Delete Layout… asks; Delete removes the layout and you land on the
      layout you used before. Greyed on Model/UV.
- [ ] Reset Layout… on UV (after changing it) asks; Cancel changes
      nothing; Reset puts UV back. Greyed on a layout you made.
- [ ] Clear Layout: the view area becomes one empty panel.
- [ ] Lock Thumb Dragging on in Model: grip drags and the middle-drag shortcut are locked. Switch to UV: not
      locked there. Back to Model: still locked.

## Layout order, favorites, and the collapsed bar (UIC-13..16)

- [ ] Run `operandi.layout.move UV 0` and check that UV moves first in
      Window > Layouts and the Layouts pie without switching layouts.
- [ ] Run `operandi.layout.star UV` and `operandi.layout.starredOnly on:true`.
      Check that the pie hides an unstarred inactive layout and still lists
      the current layout.
- [ ] Click the Operandi logo to collapse the tool set bar. Double-click
      the thin line or drag it down to bring the tool set buttons back.

## Quick switcher and pie (UIC-18, LAY-32)

- [ ] Hold Ctrl+Shift (real Ctrl on the Mac) and press Tab: a popup lists
      layouts, most recent first, the next one outlined. Tab again moves
      the outline; releasing the keys switches to it.
- [ ] Layout > Recent Layouts…: the same list; click one, or Esc closes.
- [ ] Ctrl+`: the Layouts pie; pick a slot to switch.

## Building from one empty panel (UIC-193, LAY-73, LAY-74)

- [ ] In "Blank", right-click the empty panel's top edge > Split Horizontal: two
      empty panels.
- [ ] ▾ on the left one > 3D View; ▾ on the right one > UV View.
- [ ] Quit and reopen: "Blank" opens as built.
- [ ] ▾ > All Panels lists every kind by category; side-column kinds are
      greyed with a reason. Tabbed Viewport is greyed.

## Pieces (UIC-192, LAY-72)

- [ ] In Model, set a view to Wireframe; ▾ > Save Viewport As… "Wire",
      category "Views".
- [ ] In "Blank", ▾ on an empty panel > Saved Viewports > Wire: a
      wireframe 3D view.
- [ ] Back in Model, set that view Shaded; ▾ > Save Viewport (Wire).
      Switch to "Blank": its Wire view is Shaded now.
- [ ] ▾ > Save Viewport Group As… "Pair" in a split Model; in a new blank
      layout ▾ > Viewport Group > Pair fills the view area.
- [ ] ▾ > Delete Saved Viewport > Pair: gone from the lists; panels keep
      showing what they showed.
- [ ] Quit and reopen: stars, starred-only filtering, collapsed bar, layout
      order and the switcher's recent order are as you left them.

## Hover grips (UIC-71 and UIC-72, owner-approved 2026-11-19)

- [ ] Move the pointer away from the panels. The 3D, UV and empty view dots
      stay visible. Right-click each dot and check for the view frame menu.
      Other panels' grips disappear. The Toolbox transform row starts
      immediately and Tool Tabs has no leading grip slot.
- [ ] Hover near a non-view panel's top-left corner. The grip fades in over the
      content. Click a control under the grip and check that it still works.
- [ ] Drag the grip to a docking target and check the highlight and drop.
      Repeat from the middle of a panel with Cmd/Ctrl+Shift+middle-drag.
      Press Esc before releasing and check that the layout stays put.
- [ ] Right-click a non-view panel's header or top edge. Use Float, then dock the
      panel again from its header menu. Try Left, Right, Top, and Bottom.

## Side-panel docking (J2, UIC-71, UIC-72, UIC-191, UIC-196)

- [ ] Open Model and UV. Check the stock columns, split proportions, and
      tabs. Resize both columns and drag both stacked dividers. Switch
      layouts and back, then quit and reopen. Check that the sizes return.
- [ ] Split Model into two views. Drag Tool Properties by its hover grip
      onto the bottom compass arrow of one view. Check that Tool Properties
      sits below that view and the other view still spans the full height.
      Drag the new divider to the minimum height, then lock dividers and
      check that it stays in place when dragged.
- [ ] In the stock layout, float Items. Click where the Items/Properties
      divider used to be. Check that Properties receives the input.

## Drop targets (J3, D-02..D-04, UIC-72, UIC-155, UIC-191)

The tab merge follows the [owner’s D-02/D-04 answer](SPECIFICATION_QUESTIONS.md#docking-j3-owners-docking-brief-d-02-d-04-d-12).

- [ ] Open stock Model and UV. Check that the existing panels, sizes and
      tabs match the shipped layouts and that no targets show at rest.
- [ ] Drag a panel by its hover grip. Check for a compass on every panel. Drop on each
      arrow in turn and check that only that panel splits on that side.
      Drop on the centre and check that the moving panel joins as a tab.
- [ ] Drag a panel and check for one long and one short strip on each
      side. Drop on each long strip and confirm the new pane spans the
      work area. Drop on the short strips beside Toolbox and Items and
      confirm the new pane sits beside that panel. Check below either
      short strip and just inside the viewport edge for stray targets.
- [ ] Drop on the top and bottom work-area strips and confirm docking
      at the corresponding edge.
- [ ] Drop on a tab row and check that the panel joins that group. Drop on
      horizontal and vertical dividers and check that a new row or column
      appears between the neighbours.
- [ ] Press Esc with a target highlighted, then release the grip. Check
      that the targets disappear and the arrangement and tabs stay put.
- [ ] Save and reopen after each kind of drop. Check the tree arrangement,
      split sizes, tab order and active tabs.
- [ ] With Shader Tree active in the upper group, drop that group's grip
      before the first tab of the lower group. Check that the upper tabs
      insert in order and Shader Tree comes to the front. Repeat using the
      lower compass centre and check that the tabs append.
- [ ] Bring Item List in front of Shader Tree. Hover over a viewport with
      a polygon selected, press Delete, and check that the polygon is
      deleted.

## Single-tab tear-out (D-05, UIC-72 and UIC-155)

- [ ] Drag Shader Tree within its tab row to reorder it. Drag it out to a
      viewport compass arrow. Shader Tree gets its own panel, and Items and
      Groups keep their order and the active tab where possible.
- [ ] Drag one viewport tab from a tabbed viewport onto a divider. Confirm
      the other viewport stays in its original slot.
- [ ] Start tearing out a tab, aim at a highlighted target, press Escape,
      then release. Confirm the original tabs and active tab remain in place.

## Floating and redocking (D-06, UIC-71..73, UIC-183 and UIC-187)

- [ ] Drag Shader Tree beyond the main window. Confirm only Shader Tree
      floats. Drag the title strip inside the window onto the Items tab row;
      confirm it returns at the insert mark.
- [ ] Tear out Items, dock it beside Shader Tree, then drag its grip onto
      the first position in the original tab row. Check that Items is in
      front and Shader Tree, Groups and Mesh Stack keep their order. Save
      and reopen. In Window > Panels, verify the check marks, restore a
      closed tab and bring an open floating tab forward.
- [ ] Double-click the Tool Properties grip. Drag the title strip inside
      its window onto a compass arrow, then float it again and dock onto a
      work-area edge. Check the window closes on each successful drop.
- [ ] Float a dock tab group by its grip. Drag the group's title onto a
      divider and check the tab order and front tab. Repeat a title drag
      with Escape before releasing and check the window stays open.
- [ ] Float a non-stock panel, move and resize the window, then quit and
      relaunch. Check the panel, size and position restore. Open a legacy
      layout and check its original panel and tab placement.
- [ ] Open a popover and try dragging it beyond the main window. Check
      that it never opens a native window.

## Tool Tabs panel (J6, D-07 and D-12, UIC-71, UIC-100, UIC-191, UIC-196)

- [ ] Open the stock UV layout or a saved legacy Model. Check that
      the tabs run down the right edge of the tool drawer and its shared
      tools stay above them. Hover the top-left corner of the Tool Tabs
      strip to reveal its grip.
- [ ] Drag that grip to the top and bottom work-area edges. Drag the
      adjoining divider and confirm the strip still fits exactly one
      horizontal row. Drag it to the left and right edges, then try a
      divider and compass arrow. Check orientation follows the side.
- [ ] Dock Tool Tabs above a narrow panel or shrink its width. Click both
      arrows. Check that tabs move along one row, scrolling alone leaves
      the active category alone, and clicking a revealed tab changes the
      Toolbox tools.
- [ ] Double-click the grip to float Tool Tabs. Save, quit and reopen.
      Check that its window and orientation return, then drag the title
      onto a compass arrow. Save and reopen the docked arrangement.

## Tool set bar (J7, D-08, UIC-197 and UIC-198)

- [ ] In a custom layout with two viewports, click Model, UV, and Paint
      where available. Check that the tools change and that the viewports,
      dock positions, selection, and active tool stay as they were.
- [ ] Check that Window > Layouts and Layout > Layouts show the same
      layouts and tick the current one. Switch from Window, then switch
      back from the Layouts pie. The top bar should show tool sets.

## Save, reopen and the default Model layout (J8, D-10 and D-11, UIC-190/191/194/195)

- [ ] Start with a fresh data folder. Check that the Model window shows
      Tool Tabs across the top, Toolbox above Tool Properties on the
      left, the 3D view in the centre, and Items, Shader Tree, Groups
      and Mesh Stack above Properties on the right. Check that the
      Properties panel still has its item-specific category tabs.
- [ ] In Model, resize both columns, reorder and select a sidebar tab,
      and float Tool Tabs and a viewport. Move and resize the windows.
      Save As, switch away and back, then quit and reopen. Check the
      dock tree, divider positions, tab order and selection, and both
      windows. Run the same check with a named palette already open.
- [ ] Start with a saved pre-docking Model and another user layout.
      Open both and check that they keep their original arrangements.
      Reset Model explicitly to see the new shipped arrangement.

## Selection colour (UIC-01)

- [ ] In Preferences > Display > Colors, change Accent in the default
      colour configuration. Check selected vertices, edges and polygon
      fill. Override Selection and Selection Fill separately, reset them,
      and check that each follows Accent again. Apply a built-in scheme and
      verify its explicit selection colours survive an Accent edit.

## Mode bar and message line (owner, 2026-11-19)

- [ ] A fresh Model layout shows the mode bar at the bottom, above the command line.
- [ ] Drag the mode bar's grip to the top half: it docks at the top; drag back down: bottom. Esc during the drag cancels.
- [ ] Right-click the row. Dock at Top and Dock at Bottom are the only entries. Use each to move the bar. Repeat by Cmd/Ctrl+Shift+middle-dragging the row.
- [ ] Switch layouts and back: each keeps its own mode bar position; quit and reopen keeps it.
- [ ] Status messages ("Ready", "Viewport maximised"…) show in a thin strip at the bottom of Tool Properties, not under each view; move Tool Properties and the strip follows.
