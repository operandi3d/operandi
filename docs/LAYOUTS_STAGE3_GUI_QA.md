# Layouts stage 3, batch 1: GUI check

Spec: `specifications/layouts-2026-11-16/BEHAVIOR-LAYOUTS-STAGE3.md`.
Use the QA app from `scripts/bundle-dev.sh` with its own data folder
(`OPERANDI_DEV_DATA`), never your installed app. Tick each line; note
anything odd next to it.

Batch 1 moves only the toolbox's **UV tab** onto a form. Every other tab
should look and work exactly as before.

## UV tab looks the same

- [ ] Switch to the UV layout (or type `operandi.toolbox.tab uv`). The UV
      tab shows, top to bottom: "UV Tools" header, Map drop-down, a line,
      Move / Rotate / Scale / Transform cells, a line, Unwrap..., Pack
      UVs..., Fit UVs, then Flip U | Flip V, Mirror U | Mirror V,
      Align Up | Align Down, Align Left | Align Right. Compare with the
      main QA build side by side: same sizes, spacing, text and icons.
- [ ] Click "UV Tools": the section folds; click again: it opens. Switch
      tabs and back: it stays as you left it.
- [ ] Hover each button: same tooltips as before (Unwrap... names
      uv.unwrap, the island buttons list their Shift/Alt/Ctrl meanings).

## UV tab works the same

- [ ] Map drop-down: pick another map, New Map; same as the UV view's map
      button.
- [ ] Move / Rotate / Scale / Transform start their tools; the started
      cell is lit; click it again to drop. Their shortcuts show in the
      cell corners.
- [ ] Unwrap... and Pack UVs... open their dialogs.
- [ ] With some polygons selected in a UV view: Fit UVs, Flip U, Mirror V,
      Align Left each change the UVs as one undo step (Cmd+Z undoes it).
- [ ] Shift+click Fit UVs fits each island; Alt+click each polygon;
      Cmd+click opens the Fit UVs dialog. Cmd+click Flip U opens its
      dialog.
- [ ] With Move live, the island buttons are greyed; hovering one says
      "Drop or Cancel the active tool first". Drop the tool: they light up
      again.
- [ ] Command History shows the clicks as their lines (e.g.
      `operandi.uv.preset uv.flipU`).

## Your own copy and Revert

- [ ] Quit. In the QA data folder make `Layouts/Forms/`, copy
      `crates/operandi-app/assets/forms/toolbox.uv.json` into it, and
      change "Pack UVs..." to "Pack Islands...". Start again: the UV tab
      shows "Pack Islands..." and it still opens Pack UVs.
- [ ] In that copy, change one button's "command" to `no.such.command`.
      Restart: that button is outlined and written in red; hovering says
      "Unknown command"; clicking does nothing. Nothing crashes.
- [ ] Type `operandi.form.revert toolbox.uv`: the status line says it is
      back to its shipped version; the shipped buttons show; the file in
      `Layouts/Forms` is gone.
- [ ] Put a file with `"version": 9` there instead: the shipped UV tab
      shows (the file is ignored).

## Other tabs unchanged

- [ ] Create, Deform, Sculpt, Duplicate, Edit, Vertex, Edge, Polygon
      tabs: as before (spot-check a click on each).

# Batch 2: the Select tab

Batch 2 moves the **Select tab** onto a form
(`crates/operandi-app/assets/forms/toolbox.select.json`).

## Select tab looks the same

- [ ] Open the Select tab next to the main QA build's. Same rows, gaps,
      greyed placeholders (Conditional Loop, Shortest Path, Coplanar...,
      Lazy Selection...), the three drop-downs and the Selection Sets
      section, same sizes and text.
- [ ] Fold "Selection Sets", then "Selection": Selection hides everything
      including the Sets header. Switch tabs and back: both stay folded.

## Select tab works the same

- [ ] Select a polygon: All, Invert, Connected, Grow, Shrink, Loop, Ring,
      More, Less, Boundary, Between each act as before, one undo step
      each.
- [ ] Hover Grow: "Grow · needs a selection · Shortcut: ..." with the
      current shortcut. Change Grow's shortcut in the Input Map: the
      tooltip follows.
- [ ] Convert opens a menu of Convert to Vertices / Edges / Polygons with
      their shortcuts; picking one converts the selection.
- [ ] Symmetrical is greyed ("Turn Symmetry on (X, Y or Z) first") until
      Symmetry is on; then it selects the mirror partners.
- [ ] Start Move: every selection row and Convert grey out; hovering says
      "Drop or Cancel the active tool first". Drop: they light up.
- [ ] Lasso Style: pick Rectangle, right-drag in the viewport: a
      rectangle lasso. Select Falloff and Polygon Tag stay greyed.
- [ ] Hover a placeholder: "<name> · not in Operandi yet"; clicking does
      nothing.
- [ ] Command History shows row clicks as their lines (e.g. `Grow`).

# Selection Sets rows (branch layouts-stage-3c)

The four Selection Sets rows now run their commands instead of being
greyed placeholders.

- [ ] Polygon mode, nothing selected: Assign Selection Set... is greyed,
      hover says "Select polygons before Assign Selection Set". The other
      three say "This mesh has no Polygon selection sets; use Assign
      Selection Set first".
- [ ] Select two polygons, click Assign Selection Set...: the Assign
      dialog opens; type Top, OK. Status says Assigned. Now all four rows
      are lit.
- [ ] Use Selection Set...: dialog lists Top; Replace selects the two
      polygons. One undo step.
- [ ] Create Selection Set Group...: name G, OK: status says "Selection
      set group “G”".
- [ ] Delete Selection Set...: pick Top, OK: the set is gone, the three
      rows grey again.
- [ ] Item mode: all four greyed ("... works in Vertex, Edge or Polygon
      selection mode"). Start Move: all four greyed with the tool reason.
- [ ] Apart from these four rows no longer greyed, the Select tab looks
      as before.

# Batch 3: the Deform and Duplicate tabs (branch layouts-stage-3c)

Both tabs are forms now (`toolbox.deform.json`, `toolbox.duplicate.json`).

## They look the same

- [ ] Open Deform next to the main QA build's: same rows, icons, greyed
      unbuilt rows, section headers, Quick Align / Ground Align side by
      side with no shortcut drawn. One change: Soft Selection Move (all
      four of its tools unbuilt) is greyed now.
- [ ] Open Duplicate next to main's: Duplicate Tools header, the Extend /
      Clone / Array / Duplicate titles, five flat rows with icons.
- [ ] Fold Move Tools: everything below it hides, including Rotate /
      Scale / Align / Commands. Fold Rotate Tools alone: only its rows
      hide. Switch tabs and back: folds stay.

## They work the same

- [ ] Deform: Element Move, Flex, Twist, Bend, Swirl, Push, Taper each
      start on click and light; a second click drops.
- [ ] Soft Drag flyout: click starts Soft Move. Right-click: a list of
      Soft Drag (greyed), Soft Move, Shear; pick Shear: it starts and the
      button now shows Shear. Press and hold the Bulge flyout: its list
      opens; releasing doesn't start Bulge. Pick Flare: Flare starts.
- [ ] Smooth flyout: click starts Smooth (lit); its list shows Jitter and
      Quantize greyed.
- [ ] Hover an unbuilt row: "<name> · not in Operandi yet". Hover Twist:
      its preset tooltip with the shortcut.
- [ ] Quick Align: lights while on; click again turns it off.
- [ ] Duplicate: Radial Sweep, Clone, Array, Radial Array, Mirror each
      start on click and light; a second click drops. Hover: the tool's
      tooltip and shortcut.
- [ ] Command History shows row clicks as their lines (e.g. `Twist`).

# Batch 4: the Vertex and Edge tabs (branch layouts-stage-3d)

Both tabs are forms now (`toolbox.vertex.json`, `toolbox.edge.json`).

## They look the same

- [ ] Open Vertex next to the main QA build's, in Vertex mode: Create,
      Vertex Bevel, Vertex Extrude, Merge Tool, Weld (icon), Tear, Vertex
      Slide, then Commands with the 16 rows and the Show Vertices
      checkbox; same shortcuts on the rows.
- [ ] Switch to Polygon mode: only New Vertex... and Reorder Index... grey.
- [ ] Open Edge next to main's: Edge Bevel, Edge Extrude, Extend, Bridge,
      Slide, Edge Relax, Edge Chamfer, Add Loop, Add Point, Edge Weight,
      then Commands (Grow Quads ... Edge Subdivide) with their shortcuts.
- [ ] Fold Vertex Tools, then Edge Tools: everything below hides. Switch
      tabs and back: the folds stay.

## They work the same

- [ ] Each tool row starts its tool and lights; a second click drops.
- [ ] Weld: click lights it and a Drop Weld [Q] button appears under it;
      click Drop Weld (or press Q): Weld goes dark, the button goes.
- [ ] Show Vertices: tick and untick; the view draws all points / not.
- [ ] Vertex rows: Split (one point selected) splits; Merge..., Align...,
      Join..., Set Position... open their dialogs. In Polygon mode hover
      New Vertex...: "Switch to Vertices mode first" with the row's
      tooltip under it; clicking does nothing.
- [ ] Edge: Add Loop lights, click an edge: a loop; click the row again:
      it goes dark. Same for Add Point. Bridge lights while armed.
- [ ] Edge rows: select an edge, Edge Subdivide, Collapse, Remove each do
      their thing; Set Position... opens Set Vertex Position.
- [ ] Hover rows: "Collapse · edge.collapse  [Ctrl+Backspace]"-style
      tooltips, as before.
- [ ] Command History shows row clicks as their lines (e.g.
      `Vertex Collapse`, `Add Loop`).

# Batch 5: the Polygon tab (branch layouts-stage-3e)

The tab is a form now (`toolbox.polygon.json`).

## It looks the same

- [ ] Open Polygon next to the main QA build's: Pen, Bevel, Twist Bevel,
      Extrude, Smooth Shift, Thicken, Chamfer Edit, Poly Haul, Sketch
      Extrude, Spike, Bridge, Polygon Reduction, then Commands: Make, Set
      Position..., Triple, Flip, Flip It..., Flip It Origin..., Merge,
      Remove, Collapse, Spin Quad, Unify..., the three Scale Offset /
      Flatten Ngons rows, Split, Quadruple, Detriangulate..., Align, Set
      Part..., Set Material.... Same icons, shortcuts, and the framed
      Flip / Spin Quad / Unify... / Split / Quadruple / Align buttons.
- [ ] Hold Option anywhere: Set Material... reads Set Part...; let go: back.
- [ ] Fold Polygon Tools: everything below hides. Switch tabs and back:
      the fold stays.
- [ ] Hover Pen: "Polygon Pen · unassigned" (the one wording change).

## It works the same

- [ ] Each tool row starts its tool and lights; a second click drops.
- [ ] Pen, Spike, Bridge, Polygon Reduction: click starts and lights them.
- [ ] Set Position..., Scale Offset..., Flip It..., Flip It Origin...,
      Set Part..., Set Material...: each opens its dialog and its row
      lights while it is open. Scale Offset (Detach)... opens the Detach
      dialog.
- [ ] Select a face: Triple, Remove, Collapse, Merge, Spin Quad, Split,
      Quadruple, Align, Flip, Make and Flatten Ngons do what they did;
      Unify... and Detriangulate... open their dialogs.
- [ ] Option-click Set Material...: Set Part opens. Shift-click Triple:
      it triangulates (it used to refuse modifier-clicks).
- [ ] Open the Edge tab with Set Vertex Position open: its Set
      Position... row is lit now (same command).
- [ ] Command History shows row clicks as their lines (e.g. `Triple`,
      `Polygon Merge`).

# Batch 6: the Edit and Sculpt tabs (branch layouts-stage-3f)

Both tabs are forms now (`toolbox.edit.json`, `toolbox.sculpt.json`).
Create is unchanged (left for its own batch).

## Edit looks the same

- [ ] Open Edit next to the main QA build's: Slide, Slice (six tools),
      Axis Slice with greyed Dicer / Julienne, the Center X Y Z All row,
      Subdivide (five rows), Freeze..., Booleans & Drill, Automatic
      Retopology... / Remesh..., then UV (Unwrap..., Pack UVs..., the
      island buttons). Same icons and shortcuts.
- [ ] Vertex mode: Slide shows the Vertex Slide icon; Edge mode the Edge
      Slide icon; Polygon / Item mode: greyed, "Switch to Vertices or
      Edges mode (Polygon Slide isn't built yet)".
- [ ] Hold Option: Center X Y Z read XY YZ ZX, SDS Subdivide reads SDS
      Subdivide 2X; let go: back.
- [ ] Hover Dicer: "Dicer · not in Operandi yet" (the one wording change).
- [ ] Fold Edit Tools: everything below hides; switch tabs and back.

## Edit works the same

- [ ] Slide starts Edge Slide in Edge mode, Vertex Slide in Vertex mode,
      and lights.
- [ ] Each slice tool starts and lights; a second click drops.
- [ ] Center X centres on X; Option-click Y centres on Y and Z.
- [ ] SDS Subdivide once; Option-click: twice, one undo step. Faceted,
      Smooth run; Options... and UnSubdivide... open their dialogs and
      light while open.
- [ ] Freeze..., Boolean..., Solid Drill..., Axis Drill..., Automatic
      Retopology..., Remesh... open their dialogs and light while open.
      With a tool live they are greyed ("Drop or Cancel the active tool
      first").
- [ ] Unwrap..., Pack UVs... and the island buttons work as on the UV tab.

## Sculpt looks the same

- [ ] Open Sculpt next to main's: the seven brush rows (Tangent Pinch slot
      with its corner mark), Vertex Map Tools (dim title), Paint / Smooth /
      Scale Down, a line, Erase / Set Value, Live Deformers, Weight Map,
      New Weight Map…, Remesh..., Tips (dim title) with Smooth Tip / Hard
      Tip, Lock Borders, Lock Corners.
- [ ] Hover a brush: "Push · sculpt.push · Unassigned" and its lines.

## Sculpt works the same

- [ ] Each brush starts and lights; a second click drops it. Attenuate is
      greyed with its reason.
- [ ] Hold or right-click the pinch slot: pick Center Pinch: it starts and
      the slot keeps showing Center Pinch afterwards.
- [ ] With no brush the tips are greyed ("Pick a brush first"); with one,
      Hard Tip lights when picked.
- [ ] Lock Borders, Lock Corners, Live Deformers tick and untick.
- [ ] Weight Map lists the mesh's maps; New Weight Map… adds one.
- [ ] Remesh... opens Voxel Remesh and lights while open.

# Batch 7: the Create tab (branch layouts-stage-3g, on 3f)

Every toolbox tab is a form now; Create is `toolbox.create.json`.

## Create looks the same

- [ ] Open Create next to the main QA build's: Primitives (Cube, Sphere,
      Ellipsoid, Cone / Cylinder, Torus, Capsule, Text), Draw (Arc, Tube
      with its corner mark, Pen, a gap / Curve, Bezier, B-Spline, Sketch,
      Patch), Edit (Cut, Copy, Paste, Paste Tool), Merge (Unmerge, Merge),
      Align (Quick Align, Ground Align), Measure (Ruler, Protractor,
      Dimensions, Snaps and Precision). Same sizes, icons and spacing.
- [ ] Hold Shift: every primitive cell shows an "Item" tag in its corner;
      hold Control: "Unit". Shift+Control together: no tag. Let go: gone.
- [ ] Hover Cube, then hold Shift: the tooltip reads "Cube Item · add a
      1 m primitive as a new mesh item...", then the variant list.
- [ ] Fold each section; switch tabs and back: it stays as you left it.

## Create works the same

- [ ] Each primitive starts its tool or session and lights; a second
      click drops it.
- [ ] Shift-click each primitive: a 1 m primitive as a new mesh item, one
      undo step, no drag. Control-click: into the current mesh.
- [ ] Type `Torus New Unit` and `Cube Unit` in the command line: same as
      the clicks.
- [ ] Click Tube: Tube starts. Right-click it (or hold the press about a
      second): Tube / Solid Sketch; pick Solid Sketch: it starts, the cell
      lights and wears the Solid Sketch icon.
- [ ] Arc, Text, Pen and the five curve tools start and light.
- [ ] Cut / Copy / Paste work on a selection; Paste Tool lights and pastes
      where you click.
- [ ] Merge with one mesh selected: the status line says to select two or
      more; with two: they merge. Unmerge splits.
- [ ] Ruler, Protractor, Dimensions, Quick Align, Ground Align light while
      on; Snaps and Precision opens and closes its panel and lights while
      open.

