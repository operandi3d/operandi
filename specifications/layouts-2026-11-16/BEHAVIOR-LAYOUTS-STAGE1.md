# Layouts, stage 1 (from BEHAVIOR-LAYOUTS v1.4)

Behaviour only. Stage 1 scope: one layout file for the whole main window,
shipped Model and UV layouts, switching with `layout.restore <name>`.

## Stage 1 scope

- One file describes the whole main window: side panels (toolbox, Items,
  Properties etc) join the view tree (SL-Q1), panel kinds by registry name,
  splits as fractions (LAY-71).
- Today's view-layout.json + panel-layout.json are read once and turned into
  the Model layout file. Model looks and works exactly as today.
- A second shipped file, UV (UVT-201), proves a layout is just data. The
  toolbox stays hard-coded (forms are stage 3); the layout names which
  toolbox tab opens (Model tab vs UV tab) via its open command.
- Switching: one command `layout.restore <name>` behind Layout > Layouts and
  the switcher bar tabs (UIC-11). +, stars, reorder, Save As, Revert and user
  layouts come in stage 2. Each layout remembers its state automatically.
- The Open UV Editor floating window stays as built (palettes are stage 6).
- Done when a third layout can be added by adding a file only.

## Rules

- **LAY-70** Layouts are data. Every shipped layout is a file, in the same
  format a user's own layout is saved in. Adding a built-in layout means
  adding a file, not writing code.
- **LAY-71** A layout file holds: the frame tree (splits stored as fractions
  of the window so it fits any window size, tab groups, nested groups, locks,
  minimised headers); for each slot a panel kind by name (3D View, UV View,
  Item List, Properties, Toolbox, Tool Properties, Shader Tree...) and that
  panel's own settings; the layout's name, switcher order and star; the
  command to run when it opens (e.g. open the UV toolbox set); its key map
  context (LAY-20, unused until stage 5, the field is reserved); an optional
  icon. The panel registry (UIC-157) is the list of panel kinds a layout
  file may name.
- **LAY-75** Shipped layouts are read-only files inside the app. A user's
  layouts and changed copies of shipped ones are files in the user's data
  folder: one JSON file per layout in a Layouts folder, so a layout can be
  copied to another computer by copying its file.
- **LAY-01** A layout is a named arrangement of the whole main window: the
  panel tree (splits, tab groups, sizes, locks, minimised headers), every
  viewport with its own view settings and camera angle, which toolbox set the
  left column shows, which tab is open in each tab group.
- **LAY-02** A layout is UI only: switching never changes the scene, undo
  history or selection. A layout may name one command to run when it opens
  (Model and UV use it to open their toolbox set).
- **LAY-03** Every shipped layout has the three-column shape: Left = toolbox
  (upper part) with the active tool's properties below; Centre = the views
  for that job plus the modes bar above them; Right = two stacked tab groups
  (Items/Shading/Groups/Images on top, Properties/Channels/Lists below). The
  current right column stays as it is today for Model.
- **LAY-21** Build Model and UV now. Model and UV are starred (the stars UI
  comes in stage 2; the flag is stored).
- **LAY-22** The app opens on the layout that was open when it quit. A fresh
  install opens on Model.
- **LAY-30** All ways in run one command, `layout.restore <name>`, so menus,
  bar and scripts behave identically (UIC-160).
- **LAY-31** (stage 1 part) Ways to switch: click a tab on the switcher bar;
  Layout > Layouts (every layout). Quick switcher, pie and Recent Layouts
  come later.
- **LAY-33** Switching away saves the current layout's state; switching back
  restores it exactly as left, including each view's camera angle and which
  tabs were open.
- **LAY-40** Changes are remembered automatically: moving a divider,
  re-docking a panel or changing a view's type changes that layout and
  survives switching and restarting. No Save needed.
- **LAY-41** Store a user copy of a layout only once it differs from the
  shipped default, so improvements to a shipped layout still reach users who
  never touched it.
- **LAY-49** If a panel kind a layout names no longer exists, that slot shows
  an empty panel with its name and nothing else changes. A layout file from a
  newer unknown version is ignored and the shipped default is used.
- **LAY-60** One small file per changed layout in the app's data folder
  (versioned JSON). A separate file holds tab order, stars, starred-only,
  collapsed state, MRU list and open floating windows. Shipped defaults are
  built into the app. Layouts are not saved in scene files.
- **LAY-61** Side panels (toolbox, Items, Properties) join the layout tree, so
  each layout can place them.
- **UVT-201** UV layout: Left = toolbox open at the UV tab, tool properties
  below. Centre = UV view left half, Perspective 3D view right half, 50/50,
  selection shared. Right = usual tabs, with the UV Maps list in Lists.

## UI conventions bound by this stage

- **UIC-190** Layouts are data; a shipped layout is a file in the same format
  as a user layout; adding one is adding a file, never code.
- **UIC-191** The file holds the frame tree (fractions, tabs, groups, locks,
  headers), a panel kind by registry name per slot with its settings,
  name/order/star, open command, key map context, optional icon.
- **UIC-195** A layout is the whole main window including view settings,
  camera and toolbox set; UI only, scene/undo/selection never change.
- **UIC-196** Three-column shape.
- **UIC-198** Every way of switching runs `layout.restore`; switching back
  shows the layout exactly as left.
- **UIC-209** A missing panel kind shows an empty named panel; an unknown
  file version falls back to the default.
- **UIC-220** No new button, key, menu or panel action that calls code
  directly instead of running a command line; no undo path that can't say
  which command made it.
- **UIC-221** A layout's open command is stored as a command line, as text;
  an unknown line shows red / does nothing, never crashes.
