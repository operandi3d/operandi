# Operandi UI Conventions (the bible)

Version 1, 2026-11-07. Behaviour only. Open questions and exceptions are tracked by the maintainer.

## How to use this

- Every rule has an ID (**UIC-01** ...). Cite it in specs, code comments,
  PRs and reviews: "breaks UIC-21".
- New UI follows these rules. Any departure needs the maintainer's OK.
- Tags:
  - **[ref]** the convention itself (the behaviour Operandi copies).
  - **[OP]** an Operandi detail where the convention is silent. Sensible
    default, revisable after the maintainer tries it.
  - **[Q]** open; ask the maintainer.
- Where a detailed Operandi spec already exists, it holds the details and
  this file only states the convention: colour field =
  `ui-architecture-2026-11-07/07-COLOR-NAVIGATION.md` (CN rules), modifier
  buttons = `modifier-buttons-2026-11-04/` (MB), pie menus =
  `pie-menus-2026-11-06/` (PM).
- **Colours** are theme tokens, never hard-coded. "Accent" = the theme's
  active/selected colour. "Insert line" = the theme's drop-indicator colour.
- **Keys.** "Ctrl" below is written as the convention writes it. Off Mac,
  Operandi's Cmd=Ctrl rule already makes this simple. On Mac (decided by
  the maintainer 2026-11-08): **Ctrl in any click or drag gesture is the
  real Ctrl key**, including list toggle-select (UIC-32). On Mac, Ctrl+click
  must never be turned into a right-click. Keyboard shortcuts keep the
  existing rule (Cmd on Mac), except where a Cmd shortcut clashes with the
  system (e.g. HID-12 uses the real Ctrl+H).

---

## 1. General look and feedback

- **UIC-01** One accent colour means "on / active / selected" everywhere:
  active tab label, active tool or mode button, selected list row's label,
  ticked checkbox. Nothing else uses it for emphasis. [ref]
- **UIC-02** Inactive tabs: the front tab of a group uses a light neutral
  label; tabs behind it use a darker neutral. [ref]
- **UIC-03** A menu entry or button that opens a dialog (asks something
  before acting) ends its label with "…". One that acts at once doesn't.
  [ref]
- **UIC-04** Every button shows a tooltip on hover: its name, its keyboard
  shortcut if it has one, and for some commands a one-line usage hint.
  Modifier alternates (UIC-27) are listed in the tooltip too. [ref]
- **UIC-05** Hovering a tool handle that is highlighted shows a tooltip for
  that handle, saying what dragging it does. [ref]
- **UIC-06** Context help: F1, or a Help menu entry, enters help mode. The
  cursor becomes a question mark and the next click on any control opens
  the help page for that control instead of using it. Esc or right-click
  leaves help mode without opening anything [OP]. The Help menu also opens
  the help index. [ref]
- **UIC-07** Right-click never shows an empty menu. If nothing applies to
  what's under the pointer, right-click does nothing. [ref]

## 2. Menus, layout bar, layout switching, pies

- **UIC-10** The menu bar's menus drop down, with sub-menus. They are built
  from the same form data as panels, so users can customise them the same
  way [ref]. (Form editor is future work [OP].)
- **UIC-11** Under the menu bar sits the layout bar: one tab per layout
  (workspace). Clicking a tab switches to that layout. [ref]
- **UIC-12** A "+" after the last tab creates a new layout, added at the
  right-hand end. [ref]
- **UIC-13** Drag a tab sideways to reorder; a thin insert line shows where
  it will land. [ref]
- **UIC-14** Each tab has a star toggle. A "starred only" toggle on the bar
  hides every unstarred tab. [ref]
- **UIC-15** Double-clicking a tab asks for confirmation, then resets that
  layout to its shipped default. Never reset without asking. [ref]
- **UIC-16** A collapse control at the bar's left end hides the bar,
  leaving a thin strip; clicking the strip brings it back. [ref]
- **UIC-17** A selector on the bar chooses between layout sets (e.g. the
  current set and an older one). [ref]
- **UIC-18** Quick layout switcher: a popup lists layouts in
  most-recently-used order with the next one outlined. It opens from a
  Layouts button in the bottom toolbar and from the Layout menu ("Recent
  Layouts…"); a Layout > Layouts sub-menu lists them all; a pie version
  opens on Ctrl+` (the backtick/tilde key), the real Ctrl key on a Mac.
  [ref] The switcher has **no default key**: Ctrl+Shift+Tab pages the tool
  tabs instead (Ctrl+Tab next, Ctrl+Shift+Tab previous). A user who binds a
  held chord to it in the Input Map gets the held behaviour: each further
  key press moves the outline on, releasing the keys switches.
  [Owner-approved, 2026-11-25: Ctrl+` already switches layouts]
- **UIC-19** Pie menus open from a key combination, centred on the
  pointer, with up to 8 entries in a ring. Only the first 8 entries of a
  pie's list are used, and disabled entries still take their slot. The
  default view pie is on Ctrl+Space. [ref] Details: PM spec.
- **UIC-308** A pie is a form with style pie (UIC-210). Shipped pies are
  files; user pies and changed copies of shipped ones layer over them and
  can be reverted (UIC-216). No pie is hard-coded.
- **UIC-309** A pie slot holds one command line, any line the registry
  knows, including a user alias or @macro (UIC-222), shown as text, icon
  or both; or it is empty and keeps its place.
- **UIC-310** A slot may open another pie (a sub-pie). It opens in place,
  at the same centre; Esc, or moving back to the centre and releasing,
  goes back one level. A pie can never contain itself.
- **UIC-311** Any pie, shipped or user-made, can be bound to a key through
  the Input Map (the line operandi.pie.open <id>), with the normal clash
  warning; per-layout keys apply (UIC-201).
- **UIC-312** Export Pie / Import Pie write and read one file holding the
  pie and all its sub-pies.
- **UIC-313** The pie's name sits in the centre, with its slots radiating
  out around it. A sub-pie shows its own name there.
  [Owner-approved, 2026-11-27: UIC-308..313]
- **UIC-314** A control's icon (pie slot, toolbox, toolbar or menu
  button) is chosen from a searchable grid of built-in icons or from a
  custom image file, or cleared. Pick (UIC-212) fills it with the picked
  button's icon. A custom file is copied into the user's data folder and
  travels inside exported pies and forms; a missing file shows a
  placeholder. [OP] [Owner-approved, 2026-11-27]

## 3. Tabs and buttons

- **UIC-20** Tabs stack several panels in one slot. Click a tab to bring it
  to the front. [ref]
- **UIC-21** **Tabs never switch by themselves.** Selecting something,
  starting a tool or anything else must not bring a different tab to the
  front, even if that tab now shows something relevant. Only the user
  changes which tab is visible. [ref]
- **UIC-22** A button is an icon, an icon with a label, or a label only.
  Its shortcut, if any, is shown at its right end where it fits. [ref]
- **UIC-23** An active button is drawn in the accent colour (UIC-01). [ref]
- **UIC-24** Two kinds of button:
  - **Tool / action toggles**: click again to turn the tool off. Q or Esc
    also drops the active tool. [ref]
  - **State (radio) buttons**, e.g. selection modes: clicking the lit one
    does nothing; you change state by picking another in the group. [ref]
- **UIC-25** Pop-up buttons (a button with a list or panel behind it) carry
  a small triangle in their lower-right corner. Right-click, or press and
  hold, opens the list. A quick plain click runs the button itself [OP][Q].
  Viewport-header buttons for view type and shading open their choices on
  right-click. [ref]
- **UIC-26** A preference sets pop-up transparency as a percentage, applied
  to all pop-ups. [ref] Default: opaque [OP].
- **UIC-27** Alternate buttons: holding Shift, Ctrl or Alt swaps buttons to
  their alternate label and action, live, and back on release. Alternates
  are listed in the tooltip. [ref] Details: MB spec.

## 4. Lists and trees (Item List, Shader Tree, map lists, etc.)

- **UIC-30** Click a row's name: that row becomes the only selection. [ref]
- **UIC-31** Shift+click: selects the range between the last clicked row
  and this one. [ref]
- **UIC-32** Ctrl+click: adds the row if unselected, removes it if
  selected; the rest of the selection stays. [ref]
- **UIC-33** Selected rows: darker row background, label in accent. [ref]
- **UIC-34** Selection order matters. The first-selected row is the
  primary: its row is a shade darker and its label a shade lighter than the
  other selected rows. Commands that need an order use selection order.
  [ref]
- **UIC-35** Rows with children have a disclosure arrow; children are
  indented under the parent in one nesting column. [ref]
- **UIC-36** A row's attributes (its channels or settings shown in the
  list) open with a separate +/- toggle. They sit under the row and can
  never have children of their own. [ref]
- **UIC-37** Shift+click a disclosure arrow opens (or closes) the whole
  subtree at once. [ref]
- **UIC-38** Double-click a parent row selects all of its children. [ref]
  Whether the parent stays selected too: [Q] (default: yes [OP]).
- **UIC-39** Rename in place: click to select, pause, click again; the
  label turns into an edit field. Enter commits. [ref] Esc cancels and a
  click elsewhere commits [OP]. Two clicks inside the system double-click
  time are a double-click (UIC-38), never a rename. [OP]
- **UIC-40** List search: a small "F" toggle at the list's top right shows
  a filter field. Typing filters the list live. Toggling F off restores the
  full list and keeps whatever you selected meanwhile. Options: a mode
  pop-up (plain text search, or pattern matching with wildcards [OP: `*`
  and `?`]), a Match Case toggle (off by default [OP]), and a Flat toggle
  that shows matches as a flat list for easy multi-select. Image lists
  match the original file name; other lists match the row name. [ref]
- **UIC-41** Columns: hovering a column divider shows a resize cursor;
  dragging it resizes the column. [ref]
- **UIC-42** Long choice lists (menus, item pickers) have a filter field.
  Typing narrows the list live, case-insensitively; Up/Down move the
  highlight; Enter picks. [ref]

## 5. Fields and controls

- **UIC-50** Checkbox: off = small neutral square; on = accent-filled
  square with a dark tick. Click toggles. [ref]
- **UIC-51** Numeric field: one click turns the value into an edit field
  [ref] with the whole value selected so typing replaces it [OP]; a second
  click places the caret [ref]. Enter accepts [ref]; Esc cancels and Tab
  accepts and moves to the next field [OP]. (The colour field is the one
  exception: UIC-58.) **Typing changes nothing until it is accepted**
  (Jason 2026-11-24): the scene, the viewports and the render preview
  update on Enter, Tab or a click elsewhere, never on each keystroke, and
  Esc puts the old value back. Typed text fields that change the scene
  work the same way. Dragging (the mini-slider, colour numbers, the colour
  picker) stays live. [OP]
- **UIC-52** Numeric fields accept simple arithmetic with + − × ÷
  (`+ - * /`), worked out on Enter (`360/7` gives 51.43, shown at the
  field's precision; stored at full precision [OP]). [ref]
- **UIC-53** Mini-slider: a small left/right arrow pair at the right end
  of **every** number field, whole-number fields included (tightened by
  the maintainer 2026-11-08). Only exceptions: colour fields (you drag the numbers,
  UIC-58) and read-only values that just display a number. A field greyed
  out because it doesn't apply keeps its arrows, greyed. Drag right raises the value, left lowers it.
  Click its right half to step up, its left half to step down. Holding
  Shift makes it 10× faster; holding Ctrl makes it 10× slower. [ref] One
  drag = one undo step [OP, matches mid-tool undo]. Step size: [Q].
- **UIC-54** Detents: while dragging a mini-slider the value briefly sticks
  on "nice" numbers [ref]:

  | Kind | Strong stops | Light stops |
  |---|---|---|
  | Angle | 0, every 90° | every 30° and 45° |
  | Percent | 0, every 100% | every 25% |
  | Distance | 0, powers of ten | round grid steps |
  | Plain number | 0, powers of ten | ×2, ×2.5, ×5 of each decade |

  Stickiness: strong stops hold for about 8 px of travel, light ones about
  4 px [OP].
- **UIC-55** Value presets: a small downward arrow at the far right of many
  numeric fields opens a little list of saved values for **that property
  only**. Click a value to use it. A "(new)" entry saves the current value;
  "(delete)" removes one. Saved values live in user config and survive
  restarts. [ref]
- **UIC-56** Linked multi-value fields (X/Y/Z and the like) have a link
  button left of the mini-slider. Each click cycles: Independent → Copy
  (same value to all) → Proportional (all scale by the same ratio) →
  Relative (all get the same change added) → Independent. The icon shows
  the mode. [ref]
- **UIC-57** Typing into one of those fields, how you press Enter picks the
  link mode for that entry: Ctrl+Alt+Enter = Copy, Alt+Enter = Relative,
  Ctrl+Enter = Proportional, plain Enter = the button's current mode. [ref]
- **UIC-58** Colour field: filled with its colour and showing three
  component numbers. Drag one number to change that component, with the
  fill updating live. Drag on the numbers with a modifier to change all
  three at once: Shift = brightness, Ctrl = hue, Alt = saturation. A click
  anywhere on the swatch (no drag) opens the colour picker popover, which
  is non-modal and closes when the pointer leaves it. [ref] Details: CN
  spec. Double-clicking the numbers to type a value is an Operandi
  addition (an exception to UIC-51).
- **UIC-59** One preference sets colour units everywhere colours are shown
  or typed: 0–1 decimals (default), percent, 0–255 integers, or hex. [ref]

## 6. Context menus and popovers

- **UIC-60** Right-click inside any viewport (3D, UV, …) or its header opens
  a context menu. Its entries depend on the current mode and on what is
  under the pointer (e.g. selection actions in component modes). [ref]
  See UIC-07.
- **UIC-61** Any panel can be opened as a popover and works normally inside
  it. [ref] Popover vs floating window: section 13.
- **UIC-62** A popover closes when the pointer leaves it; no click needed.
  [ref] Tolerance: leaving by more than 8 px [OP]. A pop-up opened from a
  field (colour picker, value presets) follows the same rule.
- **UIC-63** Holding Alt+Space anywhere opens the quick-access popover, with
  sections for transform, selection, action centre, tool options,
  symmetry, falloff, snapping and work plane. [ref]
- **UIC-64** A small circle at a popover's top right pins it: it stays open
  when the pointer leaves. Clicking the circle again closes it. Pinned
  popovers come back on the next launch. [ref]
- **UIC-65** A popover whose content becomes empty hides itself, and shows
  again by itself when content returns. [ref]
- **UIC-66** A tap or hold of **Space** toggles a pinned, draggable
  **Mini Properties** popover at the pointer: the active tool's short list
  of properties, or the selected item's when no tool is on. Only Space or
  its x closes it (not Esc, clicks or the pointer leaving; it overrides
  UIC-62). **Q** and **Esc** drop the active tool. **Alt+Shift+Space**
  cycles selection modes. [ref] (the maintainer, 2026-11-10, curve tool thread.)
  Its bottom-right corner drags to resize it; it keeps its top-left corner
  while you drag, and opens at that size until you quit. (the maintainer,
  2026-11-24, issue #8.)

## 7. Viewports, windows, drag and drop

- **UIC-70** Maximise: with the pointer over a viewport (or any panel,
  lists included) press keypad 0. It fills the group it lives in; keypad 0
  again over it restores the layout. The key acts on the panel under the
  pointer. [ref]
- **UIC-71** 3D views, UV views and empty view panels keep their
  always-visible top-left dot. Right-click the dot to open the view's frame
  menu. For every other panel and bar, the grip fades in over about
  0.15 seconds when the pointer enters its top-left 16-point square. It
  overlays the content and uses no layout space. Clicks still reach controls
  beneath it. Panels, tabs and bars move only in **UI build mode** (Layout >
  UI Build Mode, Ctrl+Shift+L): there every movable panel and bar has an
  accent outline, a press anywhere on it moves it (its controls and the 3D
  view don't respond; tabs still click and drag), and a bar at the top
  offers **Done**. Outside the mode a drag on a dot or grip only says how
  to turn the mode on. Drag out to tear off into a floating window.
  Other panels offer **Float** where available in the header or top-edge
  menu, which also offers the existing work-area dock targets. The mode bar
  can dock at the top or bottom only. Popovers have no grip and never tear
  off (UIC-187).
  [Owner-approved, 2026-11-20]
- **UIC-72** To dock, drag to the dock guide: a five-way cross over the
  panel under the pointer (left, right, top, bottom, centre = tab) and four
  arrows at the work area's outer edges (dock against the whole window).
  Between tabs on a tab row, an insert line shows the place. A translucent
  preview shows where the panel will land; a drop anywhere else floats it.
  The mode bar shows only the top and bottom arrows. Press Esc to cancel.
  The panel menu's **Dock at Left/Right/Top/Bottom** entries use the same
  outer-edge targets as the drag. [ref] [Owner-approved, 2026-11-20]
- **UIC-73** Floating windows (where they are, what's in them) survive
  restarts. [ref]
- **UIC-74** Dropping files onto the app [ref]:
  - a scene/mesh file opens as its own scene (several scenes can be open);
  - an image dropped onto geometry becomes a texture in that geometry's
    material;
  - an image dropped onto a 3D viewport's empty space becomes a backdrop
    item;
  - an image dropped onto any other panel is just loaded into the images
    list.
- **UIC-75** Items dragged from a 3D viewport onto a group join the group;
  dropped in the node graph, they appear there. Channels dragged from the
  channel list work the same way. [ref]
- **UIC-76** Hauling: click-drag away from every handle edits one or two of
  the tool's values with left/right (and maybe up/down) motion. An optional
  handle mode draws labelled guide lines for those values; they stay while
  the haul can be resumed, and clicking one hauls only that value. [ref]
- **UIC-77** Tools start live or wait for a click. **A tool that only
  uses your selection starts live:** picking it shows its handles straight
  away (Move, the bevels, Extrude, Smooth Shift, Thicken, Inset). **A tool
  that needs a spot from a click waits:** picking it only arms it (values
  can be typed and Apply works), and the first click in a 3D view starts it
  and supplies that spot (a cut line, a new point, the element under the
  pointer, a primitive's corner, a deformer's spine or falloff: knives,
  primitives, Bend, Twist). Every tool follows this rule; none is an
  exception. The user can change any tool's setting in the Tool Chain, and
  a preference can force one value for every tool. Picking and dropping a
  tool without a click never changes the mesh. A builder adding a tool
  applies the rule; if it's unclear which kind a tool is, log a spec
  question. [OP] (the maintainer, 2026-11-24.)
- **UIC-78** A click applies a tool that waits for Apply, and the tool
  stays live. While a tool waits for Apply (Bridge, Spike, Polygon
  Reduction), a plain left click in a 3D view, off every handle and
  without a drag, does what Apply does: the result appears as one undo
  step and the tool stays on, showing it. Until something settles it,
  changing a value or dragging remakes that same result live (more
  Segments on the bridge just made), still one undo step. Undo removes it
  and the tool stays on with its values; a new selection or another edit
  settles it, and so does another Apply or click, which applies again to
  what is selected now (Spike spikes again; Bridge has nothing left to
  join and says so). Drop (Q) keeps the result and turns the tool off;
  Cancel removes only a result that is still live. Polygon Reduction's
  result is live before Apply, so its click keeps a changed preview and
  the next drag reduces again (RED-004). Clicks never select while such
  a tool is on: select first, then pick the tool; for another, drop it,
  select, and pick it again. Drags still drive the tool (UIC-76);
  Shift-click, middle-click and right-click keep their meanings, and
  clicks a tool already uses keep their job (UIC-77 click starts, slices,
  Duplicate and Paste placing, Pen, Tube and Path points). If the tool
  can't apply, the click changes nothing and the status line says why.
  [OP] (the maintainer, 2026-11-25.)
- **UIC-306** Grow and Shrink keep a transform tool live. While Move,
  Rotate, Scale or Transform is on, Grow (Shift+Up) and Shrink
  (Shift+Down) change the component selection without dropping the tool:
  the drag so far stays as its own undo step, the selection change is the
  next step, and the tool starts again on the new selection with a zero
  offset and its handle placed by the current Action Center, as when
  another item is clicked in the Items tree. Clicks in a 3D view still
  never select while a tool is on (UIC-78); other tools follow UIC-307.
  [OP] (Jason 2026-11-27.)
- **UIC-307** Tools aren't modal. While a tool is on, every panel, menu,
  Properties field, toolbar button and shortcut command still works; a
  tool never makes the rest of the window ignore input. If the tool hasn't
  changed anything yet, the command runs and the tool stays on. If it has,
  its edit settles first as its own undo step, the command runs as the
  next step, and the tool stays on and starts again from zero on what is
  selected now (as in UIC-306). A command that makes a new mesh item (N,
  Add Item) moves the tool to the new mesh; one that makes a non-mesh item
  (locator, camera, light) leaves the tool on the mesh it was on. Only the
  3D view is reserved: clicks there still never select while a tool is on
  (UIC-78) and drags still drive the tool (UIC-76). A command that can't
  run with the tool on drops the tool first, and the status line says so.
  Builders don't add a "drop the tool first" guard to a panel or command;
  if one seems needed, log a spec question. [OP] (Jason 2026-11-27, from
  issue #36.)

## 8. Dialogs (command option boxes)

**Status: ADOPTED by the maintainer, 2026-11-08 12:21Z.**

What this covers: the small box a command opens to ask for options before
it runs (the "…" commands of UIC-03), plus confirmations. It does **not**
cover tool properties (those live in panels) or big editors.

**The one-line picture:** a compact dark box; a two-column form on the
left (labels right-aligned, fields in one column); a thin divider; a
narrow column on the right with OK on top and Cancel under it, each
showing its key. Nothing else.

### Frame
- **UIC-80** A command whose label ends in "…" opens a dialog before it
  runs; a command without "…" never does. [ref]
- **UIC-81** The window is small and modal, with the normal system title
  bar. The title is the command's name, as in the menu, without the "…"
  [ref]. It's sized to fit its content exactly: no resize, no scrolling,
  no empty space below the last row [ref]. It opens centred over the main
  window [OP].
- **UIC-82** Inside there is **only** the form and the button column. No
  heading, no description paragraph, no icon, no group boxes, no
  bottom button bar. Help for a field goes in its tooltip (UIC-04). [ref]

### Button column
- **UIC-83** Buttons sit in a narrow column on the **right**, separated
  from the form by a thin vertical divider, starting at the **top**. [ref]
- **UIC-84** OK is first, Cancel is directly under it. Each button spans
  the column's width; its label is left-aligned, and its key is shown
  right-aligned in dim text: **"Ret"** on OK, **"Esc"** on Cancel. [ref]
  Any extra buttons (rare: e.g. Apply, Reset) go below Cancel, same style
  [OP]. The column is 70 wide (UIC-94) unless a label and its key don't
  fit: then the whole column grows to the widest button, so every button
  stays the same width and no label runs past its edge (e.g. "Skip This
  Build") [OP, the maintainer 2026-11-27].
- **UIC-85** Keys: Enter = OK and Esc = Cancel, wherever focus is in the
  dialog. [ref] While a field is being typed in, the first Enter only
  commits that field; the next Enter is OK [OP][Q].

### Form
- **UIC-86** One control per row, in two columns: **labels right-aligned**
  against a shared edge, **fields left-aligned** in one column to its
  right. The label column is as wide as the longest label; every field is
  the same width, filling the space up to the divider. [ref] So labels hug
  their fields, not the dialog's left edge: only the longest label reaches
  the left padding, and shorter ones start further right, with empty space
  to their left (e.g. "Axis" sits right next to its field). [ref] Labels are
  short Title Case text with no colon [ref].
- **UIC-87** **Checkbox rows have no left label.** The box sits at the
  start of the field column and its text sits right of the box. Look per
  UIC-50 (dark square off; accent square with dark tick on). [ref]
- **UIC-88** **Choice from a list:** a full-width dark field showing the
  current choice on the left and a small down-triangle on the right; click
  opens the list (long lists get the filter of UIC-42). [ref]
- **UIC-89** **Choice from a few short options** (2–4, like X / Y / Z):
  a full-width row of equal joined segments; the chosen one is filled in
  accent (UIC-01), the rest are plain field colour. [ref]
- **UIC-90** **Numbers:** the standard numeric field (UIC-51..57): value
  left-aligned **with its unit** ("5.0 °", "1 mm", "0 m"), mini-slider
  arrows at the right end. [ref]
- **UIC-91** **Text:** the same dark field, text left-aligned. [ref]
- **UIC-92** **Options that don't apply grey out in place.** When another
  setting makes a row irrelevant, its label and field dim and stop
  responding, but the row stays exactly where it is. Rows never appear,
  disappear or move as settings change. The greying updates live as the
  user changes the controlling setting. [ref]
- **UIC-93** Rows are ordered so a controlling option sits **above** the
  rows it greys out [OP; matches every example seen].

### Look and size
- **UIC-94** Same colours, font and font size as the property panels:
  dialog background = panel background; fields a darker inset with small
  rounded corners; no shadows or gradients inside. [ref] Numbers below are
  Operandi's [OP], at 1× scale:

  | Thing | Size |
  |---|---|
  | Row height (field, checkbox, segment row) | 20 |
  | Gap between rows | 3 |
  | Outer padding (all sides) | 8 |
  | Gap label → field | 8 |
  | Field column width | at least 200 |
  | Divider gap (each side) | 6 |
  | Button column width | at least 70; wider to fit the longest label (UIC-84) |
  | Button height / gap between buttons | 20 / 4 |
  | Checkbox box | 14 square |

### Behaviour
- **UIC-95** The dialog opens with the values used last time for that
  command (first time: the command's defaults). [OP][Q: this session only,
  or across restarts? Default: across restarts.]
- **UIC-96** OK runs the command once = **one undo step**. Cancel (or
  closing the window) changes nothing. [OP, matches mid-tool undo rules]
- **UIC-97** Confirmations ("Reset this layout?") use the same frame: the
  message as plain text in the form area, OK/Cancel column on the right.
  [OP]

### How builders keep to it
- **UIC-98** All dialogs are built with **one shared dialog component**.
  A command lists its rows (label, kind, default, unit, and an "enabled
  when …" condition) and the component does all layout, keys, greying and
  sizing. Hand-laid-out dialogs and generic toolkit windows are not
  allowed. [OP]
- **UIC-99** Review checklist for any new dialog: title = command name ·
  buttons right, OK over Cancel, Ret/Esc shown · labels right-aligned, no
  colons · checkboxes with text on the right, no left label · units in
  numeric fields · irrelevant rows greyed, never hidden · no headings,
  descriptions or bottom bar · window fits content · every number field
  has its drag arrows (UIC-53). [OP]

## 9. Property panels (item properties, tool properties, settings)

**Status: ADOPTED by the maintainer, 2026-11-08 12:35Z.**

What this covers: every panel made of rows of settings: item Properties,
Tool Properties, viewport/scene/render settings, preference pages, the
gear popovers. Dialogs (section 8) use the same row rules but a different
frame.

**The one-line picture:** a dark column of compact rows. Labels are
right-aligned against one shared edge; next comes a narrow column of
small status circles; then one column of fields that are all the same
width. Rows are grouped into collapsible sections with darker header
bars. Tabs run down the right edge with their names turned sideways. A
small header row at the top holds the preset pop-ups.

### Frame and tabs
- **UIC-100** Tabs for the panel's pages sit in a narrow **vertical strip
  on the right edge**, the full height of the panel. Tab names are rotated
  90° and read top to bottom. They are full names, never shortened. The
  active tab's name is in accent (UIC-01); the others are plain grey.
  Click a tab to show it. [ref]
- **UIC-101** Which tabs appear depends on what's selected: item
  properties show one tab per aspect of the selected item(s) (e.g. the
  item itself, its surface, display, tags). Tabs never switch by
  themselves (UIC-21). If the shown tab vanishes because the selection
  changed, the first tab is shown [OP].
- **UIC-102** A thin vertical scroll bar sits between the content and the
  tab strip. Content that doesn't fit scrolls; it never flows into a
  second column and never overlaps. [ref]
- **UIC-103** Header row at the top of the content, right-aligned: three
  small text pop-ups, each a word plus ▾: **Presets**, **One Tab**,
  **All Tabs**. [ref] They hold **presets** for the panel's values
  ("All Tabs" saves values for every tab, for example as the defaults for
  new items of that type; "One Tab" does the same for the current tab
  only). [ref, confirmed by the maintainer 2026-11-08] Menus as seen [ref]:
  - **One Tab ▾**: any saved presets for this tab, then "(new for <item
    type>)", "(rename)", "(delete)". Rename and delete are greyed when
    there's nothing to act on. Same shape as value presets (UIC-55).
  - **All Tabs ▾**: a find field at the top; any saved presets; "(new, all
    tabs for <type>)", "(new, selected tabs for <type>)", "(rename)",
    "(delete)"; divider; "Load Default for <type>", "Save Default for
    <type>" (saved defaults are used for new items of that type); divider;
    "Load Default Hauling for <type>", "Save Default Hauling for <type>"
    (later, with hauling). "Load …" is greyed until a default has been
    saved. All of it is greyed unless exactly one item is selected.
  How the tabs
  are shown (one at a time or merged into one scrolling page) is a view
  option on the O key (ui-architecture 03), not this header. [ref]
- **UIC-104** Item properties start with a **Name** row (text field)
  above all sections. It is shared by every tab, and is shown once
  when tabs are merged. [ref]

### Sections
- **UIC-105** Rows are grouped into **sections**. A section header is a
  full-width bar, darker than the rows and about 18 px tall, with a
  disclosure triangle at the left and the section name. Clicking anywhere
  on the bar folds or unfolds the section. [ref]
- **UIC-106** Sections start open, except a few rarely used ones (e.g.
  Notes) that start folded. Fold state is remembered per section type
  across restarts. [ref; persistence OP]
- **UIC-107** Sections follow each other directly. There are no gaps,
  boxes or frames around them. Inside a section, a half-row of space may
  split small groups of rows. [ref]

### Rows
- **UIC-110** One setting per row, about 20 px tall. Every row is laid out
  on the same three columns: **label | status circle | field**. [ref]
- **UIC-111** **Labels are right-aligned** against one edge, which is
  shared by every section of the visible tab. The label column is as wide
  as the tab's longest label, so labels hug their fields (the same rule
  as UIC-86). A label longer than about 45% of the panel width is cut off
  with "…", and the full text goes in its tooltip [OP]. Labels are Title
  Case with no colon. [ref]
- **UIC-112** **Channel status light:** a small circle sits between label
  and field on every row whose value is a channel (something that can be
  keyed, linked or driven). It is the channel's status light, the same
  marks the Channels list shows in its status column [ref]:
  - **none**: at its default value (plain grey ring);
  - **constant**: set to a non-default value, not animated;
  - **animated**, no key on the current frame; **key** on the current
    frame (red);
  - **driven** by another source (a link, modifier or animation feed);
    **driver** (an arrow over the dot: this channel drives others);
  - **mixed** (several items selected with different states);
    **undefined**.
  Clicking it sets or removes a key at the current time; clicking the row's
  label selects the channel (UIC-119). A preference can hide the whole
  column. [ref] **Operandi now:** only "none" and "constant" can happen
  until keys and links exist, so only those two show; clicking does
  nothing yet [OP]. Rows that aren't channels have no circle, but their
  fields still start at the same field-column edge, so every field lines
  up. [ref; column alignment OP]
- **UIC-113** **Fields** all share one column and one width, filling to
  the right margin. [ref] Kinds, all as already defined:
  - number: UIC-51..57, value left-aligned **with its unit** ("0 m",
    "0.0 °", "100.0 %"), mini-slider arrows at the right end. **No long
    sliders.** [ref]
  - choice: a dropdown filling the field (UIC-88). In panels, every
    multiple choice is a dropdown; joined-segment rows (UIC-89) are for
    dialogs and tool properties only [ref, per the viewport-panel spec].
  - text: plain field. Colour: UIC-58.
- **UIC-114** **Vector rows** (position, rotation, scale and the like):
  one row per component. The first row's label is the full name plus the
  axis ("Position X"); the next rows are labelled with just the axis
  ("Y", "Z"), right-aligned like every label. Each row has its own status
  circle and field, and the link button of UIC-56 sits before the
  mini-slider. There's no blank row inside the group. [ref]
- **UIC-115** **Checkbox rows have no label in the label column.** The
  status circle stays in its column, the box sits at the start of the
  field column, and the text sits to its right. Text too long for the
  field is cut off with "…" and shown in full in the tooltip.
  Consecutive checkboxes make a neat stack. [ref]
- **UIC-116** **Buttons inside a panel** (e.g. Reset, Freeze, Add, Browse…)
  fill the field column's width, with no left label, the text left-aligned
  and stacked one per row. A button that opens a list shows ▾ at its
  right end (pop-up button, UIC-25). A button that belongs to a field sits
  on the row right under that field (e.g. a file path field, then
  "Browse…"). [ref]
- **UIC-117** **Greyed, never hidden:** a setting that doesn't apply
  (because of another setting, or because it is read-only) keeps its row.
  Its label, circle and field dim and stop responding, and this updates
  live. [ref] Exception: features not built yet are left out entirely,
  not shown greyed [OP, per viewport-panel spec].
- **UIC-118** **Several items selected:** a row edits all of them
  (UIC-56/57 apply). Where their values differ, the field shows "(mixed)"
  until it is edited [OP][Q].
- **UIC-119** Every row has a tooltip (UIC-04). Clicking a row's label
  selects that channel (for channel tools and keying later) [ref, per
  ui-architecture 08].

### Look
- **UIC-120** Same colours and font as the rest of the UI: panel
  background, fields a darker inset with small rounded corners, section
  bars darker than both. Sizes, at 1× scale [OP]:

  | Thing | Size |
  |---|---|
  | Row height / gap between rows | 20 / 2 |
  | Section header height | 18 |
  | Gap label → circle → field | 4 · 4 |
  | Status circle | 9 across |
  | Right margin before scroll bar | 6 |
  | Vertical tab strip width | 22 |
  | Header row (Presets…) height | 18 |

### How builders keep to it
- **UIC-125** All property panels are built with **one shared form
  component**: the same one dialogs use (UIC-98), in "panel" mode. A
  panel only declares data: tabs → sections → rows. Each row gives its
  label, kind, unit, default, an "enabled when …" condition, whether it's
  a channel (for the circle), and a proficiency tag (ui-architecture 03).
  The component does all layout, alignment, greying, folding, tabs and
  scrolling. Hand-laid-out panels and toolkit default widgets are not
  allowed. [OP]
- **UIC-126** Review checklist for any panel: tabs on the right, turned
  sideways, full names, accent on the active one · Presets / One Tab /
  All Tabs header · Name row first (items) · darker foldable section bars ·
  labels right-aligned on one edge, no colons · circles column lined up ·
  every field the same width · units shown · dropdowns for choices, no
  sliders · checkboxes with text on the right, no left label · buttons
  full field width · irrelevant rows greyed, never hidden or moved ·
  one column, scrolls, no overlap · every number field has its drag
  arrows (UIC-53). [OP]

## 10. Tree lists (Items, Mesh Stack, Shader Tree, Groups)

**Status: ADOPTED by the maintainer, 2026-11-08 12:35Z.**

What this covers: every panel that shows a tree of things with columns:
Items, Mesh Stack, Shader Tree, Groups, and any later ones (channels,
images, vertex maps). Section 4 (UIC-30..42) already covers selecting,
expanding, renaming and searching in them; this section covers how they
**look and are built**.

**The one-line picture:** a tool row on top; a header row with small
icon columns on the left, then Name, then any extra columns; thin
vertical lines down every column; a tree with dotted connector lines,
type icons and names; the selected rows darker with names in accent.

### Shared shape
- **UIC-130** All tree lists are built with **one shared tree-list
  component**. A panel declares only: its tool row contents, its columns,
  and how to get rows (icon, name, style, children, cell values). The
  component draws everything and handles selection (section 4), expand,
  rename, drag and drop, scrolling, column resizing and the search row.
  Hand-built lists are not allowed. [OP]
- **UIC-131** Top to bottom, every tree list has: **tool row(s)** →
  **column header row** → **tree body** → horizontal scroll bar. There is
  a vertical scroll bar at the right when needed. [ref]

### Tool row
- **UIC-132** The tool row is one compact row (two at most). On the
  **left** is the panel's main "add" control, a wide pop-up button with
  ▾ ("Add Item", "Add Operator", "Add Layer"); a plain button if there is
  nothing to choose ("New Group"). On the **right** are small text
  buttons and icon toggles, with the **funnel (text filter) toggle always
  last, at the far right**. It opens the search row of UIC-40. [ref]
- **UIC-133** A setting the tool row needs in order to make sense is
  shown as a small label and a dropdown on the same row ("View [Shader
  Tree ▾]", "Filter [(none) ▾]"). [ref]
- **UIC-134** Icon toggles in the tool row are lit in accent when on
  (UIC-01) and have tooltips (UIC-04). [ref]

### Columns
- **UIC-135** The header row names each column. **State columns** come
  first, at the left: they are narrow, with an icon instead of a word as
  their header (eye = visibility, plus whatever the panel needs). Then
  comes **Name**, which takes the spare width. **Value columns** come
  last, on the right, headed by a short word or symbol ("%", "B",
  "Effect", "#"). [ref]
- **UIC-136** A thin vertical line runs down each column boundary for the
  **whole height of the panel**, even below the last row and in an empty
  list, so the columns always read as columns. [ref]
- **UIC-137** Column widths can be dragged (UIC-41). Value columns cut
  long text off with no ellipsis when narrow ("Final Co"); names cut off
  with "…" [OP].
- **UIC-138** **Eye column:** one click toggles that row. Dragging down
  the column toggles every eye it passes over to the same new state.
  Alt+click shows only this one (click again to bring the rest back). [ref]
  Other state-column icons work the same way: click to toggle, and the
  icon is in accent when on. [ref]

### Tree body
- **UIC-139** Rows are about 17 px tall: the compact list height, a bit
  smaller than form rows [OP]. Each row reads: tree
  indent → disclosure triangle (if it has children) → type icon → name.
  [ref]
- **UIC-140** **Connector lines:** thin dotted lines join each child to
  its parent's column, with a small mark where they meet. They show the
  hierarchy even when indents are small. [ref]
- **UIC-141** **Name styles carry meaning** [ref]:
  - selected: darker row, name in accent (UIC-33, with the primary
    selection per UIC-34);
  - normal: plain text;
  - empty (e.g. a mesh with no geometry): light grey name;
  - top-level scene/root row: **bold**, with "*" after the name when it
    has unsaved changes;
  - instances: *italic* (with a tinted icon);
  - disabled/hidden rows: dimmed name [OP].
- **UIC-142** A row can carry a user-chosen **editor colour**, drawn as a
  tint behind the row (12 colours to choose from). [ref]
- **UIC-143** Drag rows to reorder or reparent them. Dropping **on** a row
  makes the dragged row its child; dropping **between** rows places it at
  that level. A thin insert line shows a between-drop, and the target row
  is outlined for an on-drop. [ref] (Details: drop-action spec.)
- **UIC-144** Right-click a row for that row's context menu (UIC-60);
  right-click empty space for the panel's general menu (or nothing,
  UIC-07). [ref]
- **UIC-145** Empty list: header and column lines still show, and the
  body is blank, with no "nothing here" message [ref].

### Per-panel differences (everything else is shared)
- **UIC-150 Items:** tool row = Add Item ▾ · Select · Filter · icon
  toggles · funnel (item-list-header spec). State columns = visibility,
  item/component edit mode, world/local space. No value columns. [ref]
- **UIC-151 Mesh Stack:** the "add" control is Add Operator ▾; the right
  side holds the evaluation controls (pause / step) and view toggles,
  then the funnel. State column = enable eye. Value column = "#" (order
  number). The top row is the mesh item, and its operators nest under it.
  [ref]
- **UIC-152 Shader Tree:** two tool rows: (1) View [▾] + Assign Material;
  (2) Filter [▾] + Add Layer ▾ + one icon. State column = enable eye.
  Value columns = blend mode ("B"), opacity ("%"), and effect ("Effect",
  what the layer drives). The root render row may also show a +/- for its
  attribute rows (UIC-36). [ref] (Details: shader-tree specs.)
- **UIC-153 Groups:** tool row = New Group · Filter field · funnel.
  State columns = visibility and one group-type column. [ref]
- **UIC-154** A new tree list must fit the shared shape. If it needs
  something the component can't do, add it to the component (and to this
  section), never as a one-off. [OP]
  - *Fill-last-column option (the maintainer, 2026-11-16):* a list may opt
    in so its last column takes the leftover width and shrinks in narrow
    panels, the Name border drags down to 40 px, and cut-off cells show
    their full text in a hover tooltip. First used by Command History. [OP]

### Panel-group tabs (above any list)
- **UIC-155** When several panels share a slot, their tab names run along
  the top. A name that doesn't fit is cut off with "…" ("Mesh …"). When
  the tabs overflow, a ▾ at the end lists the rest. [ref]
- **UIC-156** A **+** after the last tab adds a panel to the group. It
  opens a menu of panel types sorted into categories, each a sub-menu:
  3D views/editing, toolbars, properties, data lists, info, editors,
  utilities, application. [ref] Picking one adds a new tab with that panel
  at the end of the group and brings it to the front (the user asked for
  it, so this doesn't break UIC-21). [ref/OP] Tabs can be dragged to
  reorder (insert line, as UIC-13) and closed from their right-click menu
  [OP]. The group's tabs are saved with the layout. [OP]
  Exception (the maintainer, 2026-11-22, for now): a docked panel's header
  also shows a × that closes it (added for the Node View); revisit in the
  bible rework.
  Change (the maintainer, 2026-11-23): dragging a tab sideways **within its
  own tab row** reorders it at any time, outside UI build mode. Dragging a
  tab **out** of its row (to dock or float it) still needs UI build mode,
  like every other panel move. Tool tabs (Create, Select…) are not
  reorderable.
- **UIC-157** **Plumbing:** every panel kind registers itself once in a
  panel-type registry (name, category, how to create it). The + menu, the
  layout save/load and any later "new window" command all read that
  registry. Categories with no registered panels are hidden. Building a
  new panel = registering it, with no menu edits. [OP] (Ties in with the
  layouts registry, ui-architecture 01.)

## 11. Commands (everything is scriptable)

Added 2026-11-09, the maintainer's OK in the extensibility thread (plan:
`extensibility-2026-11-09/EXTENSIBILITY-PLAN.md`, decision D4). This
widens the older builder rule in the repo's AGENTS.md ("panel controls,
shortcut and command"), which builders read as covering modelling tools
only.

- **UIC-160** Every user action runs a **named command** that you can also
  type in the command line, and that scripts, macros and the MCP bridge
  can run, with the same result and the same undo step. That covers
  buttons, menu and pie entries, shortcuts, list actions (rename, delete,
  reorder), dialog OK and every property field change. It applies to
  everything (lights, cameras, shader tree, textures, viewport options,
  preferences), not just modelling tools. Click-only is a bug. Exceptions
  need the maintainer's OK. [OP]
- **UIC-161** Live mouse gestures (orbit, a handle drag, a brush stroke)
  are not themselves commands, but what they produce must be: the tool and
  its values, the view change, the edit. A recorded macro of the gesture
  replays its result. The result is recorded as command lines (tool on,
  tool values, apply, tool off), see UIC-224. [OP]
- **UIC-162** The command takes as arguments everything the UI would ask
  for. A dialog's fields are its arguments; the dialog opens only when
  they are missing (UIC-80). Command, tool and argument names follow the
  name map (`operandi-core/src/commands/command_names.rs`, the maintainer 2026-11-06). [OP]
- **UIC-163** Anything a panel shows can also be **asked for** by a query
  command, so scripts can read as well as change it. [OP]
- **UIC-164** **Enforcement:** an automated test fails when a menu entry,
  toolbar or toolbox button, or property row has no command behind it.
  Every GUI QA checklist includes typing the command once. Features that
  are click-only today are backfilled during the command-list work
  (extensibility plan Phase 0); until then they are a known gap, not a
  precedent. [OP]
- **UIC-165** **No invented commands.** A property field change runs one
  of the general value setters (item value, preference, tool setting,
  shader layer value); it does not get a command of its own. A new named
  command is added only when the reference app has one in the name map
  (`command_names.rs`, `command-names-2026-11-06/`) or with the maintainer's OK.
  A value the reference app never let you change from a script does not need a
  command. UIC-164's test counts a control as covered when it runs a
  general setter. Also (the maintainer, 2026-11-09 14:55, from the command-coverage
  check a command-coverage check):
  - Friendly typed names ("Top View", "Select More") are **aliases** of a
    reference-named command, never the only name, so macros and command history
    show the reference name.
  - A command for an Operandi-only feature (no reference-app counterpart) goes under
    `operandi.*` (or keeps an existing Operandi name) and says so in the
    registry.
  - Window chrome (divider locks, tear-off, float, compact layout, split or
    close view) may stay click-only; list each for the maintainer.
  (the maintainer, 2026-11-09, command-list thread.) [OP]

## 12. Rendering: what "matching the reference app" means

Added 2026-11-13, the maintainer's standing rule.
Applies to every render, material, light,
camera and environment feature, and to every render comparison.

- **UIC-170** **Cycles is the source of truth for physics.** How light
  bounces, refracts, scatters and adds up is whatever Cycles does. No
  fudges layered on top of Cycles to imitate the reference app's look (no
  hidden multipliers, fake glows, tone tweaks or special cases). If a
  render looks wrong, the fix is a real bug fix (units, a missing
  feature, a wrong setting, colour handling), never a cosmetic nudge. [OP]
- **UIC-171** **The reference app guides workflow, not physics.** Tools, panels, the
  shader tree, names, defaults and units follow the reference convention, so a
  reference-app user knows where everything is and what a number means. [OP]
- **UIC-172** **Match the intent, not the pixels.** The same scene (same
  lights and intensity units, same material settings, same colour
  handling) should give a render a reference-app user recognises: same ballpark,
  same mood. Pixel-exact matching is never a goal and never a test. [OP]
- **UIC-173** **Comparisons use a "close enough" bar.** Side-by-side
  renders against the reference app exist to catch real bugs: wrong units or scale,
  missing features, wrong colour space, a setting ignored. A difference
  that comes from Cycles being a different (correct) renderer is not a
  bug. Comparisons stay research-side; builders get a written fix list
  only (clean room). [OP]
- **UIC-174** **The shader tree is a friendly front end.** It compiles
  to ordinary Cycles / OpenPBR materials, nothing secret or Operandi-only
  underneath, so a scene's materials stay plain, portable and explainable.
  A node view of the same materials for Cycles users is a welcome later
  feature (not scheduled). [OP]

## 13. Popovers vs floating windows

Added 2026-11-14, the maintainer's OK.


- **UIC-180** Two kinds of loose panel. A **popover** is for a quick look or
  a quick change, then it's gone. A **floating window** is a panel you keep
  open while you work. Rule of thumb: one tweak = popover; open for minutes
  = window. Controls kept beside the work in a 3D view: an in-view panel
  (UIC-188). [OP]
- **UIC-181** **Popover**: opens at the pointer, or under the button or key
  that called it (O for Viewport Properties, the Snapping button, the colour
  picker). It sits inside the main window, not its own OS window, and has no
  title bar, and never tears off (UIC-187). It closes when the pointer leaves (UIC-62), on Esc, on a click
  outside, or when its key or button is pressed again. One unpinned popover
  at a time: opening another closes the first. [OP]
- **UIC-182** **Changes in a popover apply live**, as you make them. No OK,
  Cancel or Apply. Each change is one undo step (a whole drag counts as one).
  Closing it, any way, keeps the changes. Esc during a drag cancels only that
  drag. [OP]
- **UIC-183** **Floating window**: its own OS window (the maintainer, 2026-11-10: most
  floating panels float as their own OS window). It has a title bar, can be
  moved to another monitor, resized and docked (UIC-72), and remembers where
  it was and what was in it (UIC-73). It stays until you close it; Esc and
  the pointer leaving do nothing. Changes apply live here too. [OP]
- **UIC-184** **One panel, both ways.** Any panel can be either (UIC-61). It
  is the same panel with the same settings: change snapping in the popover
  and the Snapping window shows it too. How you open it picks the kind: a
  shortcut key or toolbar button opens a popover; the Window menu opens a
  floating window. Each panel's spec says which way its key and button open
  it. "Window menu" = Layout > Windows/Palettes (UIC-205). [OP]
- **UIC-185** **Pinning** (UIC-64) keeps a popover open, and it can then be
  dragged by its header strip anywhere inside the main window, but never
  outside it. It stays a popover: no title bar, no own OS window. Mini
  Properties (UIC-66) is a pinned popover with its own close rules. [OP]
- **UIC-187** **Popovers never tear off.** No grip, no dragging out into a
  floating window, no leaving the main window (this overrides UIC-71 for
  popovers). Only floating panels tear off or move outside the main window.
  For a panel on another monitor, open it as a floating window from the
  Window menu (UIC-184). (the maintainer, 2026-11-14.) [OP]
- **UIC-186** **Dialogs** (section 8) are neither: they block the app until
  you press OK or Cancel. [OP]
- **UIC-188** **In-view panel** (the maintainer, 2026-11-24): a third kind
  of loose panel, for controls you keep beside the work in a 3D view (the
  channel panel, CP-xx). It sits inside one 3D view, not its own OS window,
  with a header strip carrying its name and a close (×) at the left end;
  drag the header to move it anywhere inside the view. It stays open until
  closed: the pointer leaving, Esc and the selection changing do nothing.
  Its place is kept per view as a fraction of the view, clamped inside it,
  as layout state (UIC-73), not scene data. Changes in it apply live, one
  undo step each (UIC-182). [OP]

## 14. Layouts

Added 2026-11-16, the maintainer's OK. Detail: `layouts-2026-11-16/BEHAVIOR-LAYOUTS.md` (LAY rules), stages in `layouts-2026-11-16/BUILD-PLAN.md`.

Stages (BUILD-PLAN.md): a rule binds builders from the stage it lands in;
until then the current behaviour stays. 1: UIC-190, 191, 195, 196, 198,
209. 2: 192, 193, 194, 199, 200. 3: 197, 210, 211, 214, 215, 216.
4: 212. 5: 201. 6: 205, 206, 207. 7: 213.

### The engine
- **UIC-190** Layouts are data. A shipped layout is a file in the same
  format as a user's saved layout; adding a built-in layout means adding a
  file, never new code. [ref]
- **UIC-191** A layout file holds the frame tree (splits as fractions,
  tabs, groups, locks, headers), a panel kind by name in each slot with its
  settings, the name, order and star, an open command, a key map context
  and an optional icon. Panel kinds come from the panel registry
  (UIC-157). [ref]
- **UIC-192** Any frame or group can be saved as a named piece (Save
  Viewport As / Save Viewport Group As, with a category) and reused in any
  layout; saving the piece again updates every layout that uses it.
  Shipped layouts share one left column and one right column. [ref]
- **UIC-193** The view dot and the other panels' hover grips and header
  menus act on the panel frame (UIC-71). Frame actions include split,
  delete, detach, duplicate, save piece, revert, maximise, min header,
  and locks. The content arrow
  controls what the slot shows: any panel kind by category, Viewport Group,
  Tabbed Viewport, or panel settings. Together they build any layout from
  one empty panel. [ref] [Owner-approved, 2026-11-19]
- **UIC-194** Shipped layouts and pieces are read-only; user layouts,
  pieces and changed copies of shipped ones are files in the user's data
  folder. [ref]

### Using layouts
- **UIC-195** A layout is the whole main window's arrangement, including
  each view's settings and camera and the toolbox set. UI only: scene,
  undo and selection never change. [ref]
- **UIC-196** Shipped layouts keep the three-column shape: toolbox and tool
  properties left, views centre, Items/Properties tab groups right. [ref]
- **UIC-197** One toolbox panel, several toolbox sets. Each layout opens
  with its own set; set buttons above the toolbox switch only the toolbox.
  Click = show, Alt+click = popover, Ctrl+click = palette. [ref]
  Update (the maintainer, 2026-11-23): the set buttons live in the tool set
  bar at the top and **never change the screen layout** (no companion
  views open or close); they only swap the toolbox's tabs. Sets, in
  order: Model, UV, Brush (sculpting + painting), Setup, Animation,
  Workbench, Game. A set with one form shows it without a tab strip; a
  set with no tools shows an empty toolbox with a short note (UIC-209).
- **UIC-198** Every way of switching runs one command, `layout.restore`.
  Switching back shows a layout exactly as it was left. [ref]
- **UIC-199** Changes are remembered automatically. Save Layout, Save
  Layout As, Rename, Revert (double-click the tab; asks first), Delete
  (user layouts only; asks first), Clear Layout. [ref]
- **UIC-200** New layout asks for a name, with As Copy of Current Layout
  (off by default). [ref]
- **UIC-201** Each layout has its own key map context; keys it doesn't set
  fall back to the global map. [ref]

### Palettes and pop-up editors
- **UIC-205** A palette is a layout file in its own floating window,
  opened and closed by the same toggle. One of each; remembers size and
  place. Palettes belong to the app and stay open across layout switches.
  [ref]
- **UIC-206** The ` key hides and shows all palettes. [ref]
- **UIC-207** Texture > Open UV Editor, or double-clicking a UV map in the
  vertex map list, toggles the floating UV Editor. [ref]

### Tool panels (toolboxes and forms)
- **UIC-210** Every toolbox, toolbar and menu is a **form**: a data file of
  command buttons, sub-forms and dividers. Adding a tool button means
  editing data, never code. [ref]
- **UIC-211** Toolbox sets are tabs of forms, shared by every layout; a
  layout picks the open tab. Editing a toolbox changes it everywhere it is
  shown; a layout that needs its own uses its own form. [ref]
- **UIC-212** Workbench: Edit Workbench, Pick to Add (click any button,
  property, preset or command-history line), drag to reorder, right-click
  to set label, tooltip, icon, width, style, proficiency; dividers with
  labels make collapsible sections; add the current tool chain as a preset.
  Any form can be made a workbench. [ref]
- **UIC-213** Form Editor (System > Form Editor): tree of groups, forms and
  controls; search; Find Form by clicking the UI; properties; preview; save
  a form to its own file. [ref]
- **UIC-214** Head and tail slots: a form can add itself to the start or
  end of any other form, in sort order, without editing it. [ref]
- **UIC-215** A form or section can be shown only while a named command is
  enabled. Controls carry a proficiency level the panel can filter by.
  [ref]
- **UIC-216** Shipped forms are read-only; user edits are kept separately,
  win, and can be reverted. [ref]

### Builder rule where the reference is silent
- **UIC-209** A missing panel kind shows an empty named panel; an unknown
  layout file version falls back to the default. [OP]

Scope (not rules): Model and UV first, then Render; the all-purpose,
Schematic, Topology and Scripting layouts later; Animate, Setup, Paint, VR
and Game not built.

## 15. Command history and macros

Added 2026-11-16, the maintainer's OK. Detail: `ui-architecture-2026-11-16/` (ARCHITECTURE.md, HIS/MAC/CMD rules). Built in stages; each rule says when it starts to bind.
Until then, UIC-220 applies.

Stages: **now**; **H** = the history build (command records, history log,
Command History panel; between layout stages 2 and 3); **L1..L7** =
layout stages (section 14); **M** = macros; **P** = Python and kits.

### The bridge rule

- **UIC-220** **Don't block the future.** Until a rule below applies, new
  work must not make it harder: no new button, key, menu, pie or panel
  action that calls code directly instead of running a command line; no new
  undo path that can't say which command made the step; no new interactive
  tool session with its own private undo that couldn't emit command lines.
  A builder who can't avoid it lists it for the maintainer with the rule number.
  *Applies from: now.* [OP]

### Containers hold lines

- **UIC-221** Every container stores a **command line as text**: form
  controls (toolbox, menu, pie, workbench), key map entries, layout
  open-commands, preset rows, macros. Loaded lines are checked against the
  registry; an unknown line shows red and does nothing, never crashes.
  *Applies from: now for new containers; L3 toolbox, L5 key map, L7 menus
  and pies.* [ref]
- **UIC-222** A container may hold **any** registry line, including a user
  alias and `@macro`. No allow-lists of "tool buttons" or "key actions".
  *Applies from: L3 (forms), L5 (keys).* [ref]
- **UIC-223** A button's lit state comes from its command's `?` answer, its
  greying from the command's enable state (with the reason as the
  tooltip), and "show only when" from a named command's enable state. No
  separate UI-side logic for these. *Applies from: L3.* [ref]

### Records and history

- **UIC-224** Every run produces a **command record**: the line **as
  fired** (reference names; after a dialog, the line with the chosen values),
  class, source (typed, key, button, menu, pie, script, macro, bridge),
  success or failure message, sub-commands, time. Tool sessions record
  `tool.set <tool> on`, one `tool.attr` line per changed value,
  `tool.doApply`, `tool.set <tool> off`; a live drag records the final
  values at mouse-up, never mouse moves. Selections record by element
  index. *Applies from: H.* [ref]
- **UIC-225** Every command has a **class**: undoable, UI, side effect or
  quiet. Only undoable commands make undo steps. Queries are quiet. The
  registry test fails if a command has no class. *Applies from: H.* [ref]
- **UIC-226** Each undo step keeps the record(s) that made it, as well as
  its friendly label. Every history owner (document, live tool, extrude
  stage, primitives, pen, sketch, reduction …) does this. *Applies from:
  H.* [OP, needed for ref behaviour]
- **UIC-227** A command that runs commands (macro, preset, script, batch,
  conditional command) is **one undo step** with its lines as sub-commands.
  *Applies from: H.* [ref]
- **UIC-228** The **Command History** panel: tabs Undos, History,
  Commands, Scripts, Results, with a filter per tab and the one-line
  command entry at the bottom; collapsed to that line in the Model layout.
  Behaviour per `03-COMMAND-HISTORY.md` (HIS-xx). *Applies from: H (Undos,
  History, Commands, Results); M (Scripts).* [ref]
- **UIC-229** Undo depth pref **Max Undo Levels** (default 100); **Group
  Selection Commands** and **Max Sub-Commands Recorded** as in the
  reference. *Applies from: H.* [ref]

### Using the history

- **UIC-230** Right-click a command (Commands or History tab) → **Map
  Command to Key**, with clash warning and a context picker starting at
  Global. *Applies from: L5.* [ref]
- **UIC-231** Workbench **Pick to Add** accepts a line picked from the
  Command History. *Applies from: L4.* [ref]
- **UIC-232** **Fire Again** repeats the last command, optionally skipping
  UI and selection commands. *Applies from: M.* [ref]

### Macros

- **UIC-233** A macro is a plain list of command lines with `%1`-style
  arguments, run as one undo step. If a line fails or Esc is pressed, the
  whole macro is undone and the message names the line (DEV-4, approved
  11-16). *Applies from: M.* [ref + OP]
- **UIC-234** Recorder at the right of the command entry: Record/Stop,
  Replay (hold for Replay Repeat), Options (save to file, store, record UI
  commands: Automatic (default), Always, Never). Only top-level lines are
  recorded (a script is one line); queries and quiet commands never; Undo
  removes the undone lines from the recording. *Applies from: M.* [ref]
- **UIC-235** Macro files use the reference's `.lxm` text format
  (`#LXMacro#` first line, `#` comments). Stored macros are files in the
  user Macros folder (DEV-2, approved 11-16); shipped and kit macros are
  read-only (lock). *Applies from: M.* [ref + OP]
- **UIC-236** A macro runs from anywhere a line is accepted with
  `@<name> args`. "Save a macro as a command" = a **user alias** naming it;
  aliases can't take over a built-in name (DEV-3, approved 11-16).
  *Applies from: M.* [ref + OP]
- **UIC-237** Scene files never hold macros or scripts that run on open
  (extensibility D5). *Applies from: now.* [OP]

### Layers

- **UIC-238** Macros, aliases, key maps and forms follow the layouts'
  storage rule: shipped read-only, user layer on top, Revert drops the
  user copy (UIC-194). *Applies from: as each lands.* [ref]
- **UIC-239** Every GUI QA checklist that adds a control includes: "the
  action shows in the Command History with its line". *Applies from: H.*
  [OP]

## 16. Future doors

These keep three planned features cheap to add later: project-aware files
(being built now), a second selection style, and a node material editor.
Like UIC-220, they forbid new work that would make those harder.

- **UIC-240** **No feature keeps its own file path.** Every file a scene
  uses (pictures, textures, light profiles, node-graph images, and any
  future kind) is added through the one scene file-reference path and stored
  in its table or fields. A new saved field that holds a raw path is a
  bible break.
- **UIC-241** **Paths inside a project are saved relative to it.** On save,
  a file inside the scene's project is written relative to the project
  root, so a copied or moved project still works. A file outside the
  project keeps its full path. Scenes saved before this rule still open.
  Saving never moves, renames or deletes the user's files.
- **UIC-242** **Files land in the project.** With a project set, a file the
  user adds from outside the project is copied into the project's folder
  for its kind (backdrop pictures → `references/`, textures and other
  images → `sourceimages/`), and the scene points at the copy. The original
  is never touched. Renders and exports default to the project's `renders/`
  and `exports/`. Can be turned off in Preferences.
- **UIC-243** **Selection is one service.** A viewport click, lasso,
  double-click or list row becomes selection commands in one place per
  input kind. Tools never decide by themselves what the user picked, so a
  second selection style (for example objects first) can be added as a
  setting later without touching every tool.
- **UIC-244** **Materials stay graphable.** Whatever the Shader Tree holds
  must always be expressible as a Cycles-style node graph, and renderers
  and viewports read only that combined result, never raw tree layers. The
  Shader Tree is one view of a material and a future node editor will be
  another, with one source of truth between them, never two copies that
  can disagree.

Added 2026-11-16. *Applies from: now.*

---

## 18. User Guide

The User Guide is the HTML manual at operandi3d.org/guide/. Full rules:
[docs/guide/RULES.md](guide/RULES.md). These are the rules builders cite.
(Section 17 is reserved for the UI map names, UIC-260..285.)

- **UIC-300** **No page, no merge.** A branch that adds or changes anything
  a user can see or do ships its guide page (new or updated) in the same
  branch, or says "no docs needed" in its merge note. [OP]
- **UIC-301** **Help id = command name.** Every tool, command, panel and
  preference uses its command-registry name as its help id. Guide pages list
  the help ids they cover; the app only ever opens
  `/guide/<version>/go/<help-id>`. [OP]
- **UIC-302** **Help > User Guide opens the guide for the running version**
  (test builds open `/guide/test/`). F1 help mode (UIC-06) opens the clicked
  control's page by its help id. [OP]
- **UIC-303** **UI names in the guide are spelled exactly as on screen**, in
  bold, using the approved UI map names. If the guide and the app disagree,
  one of them is a bug. [OP]
- **UIC-304** **Screenshots come only from Operandi**, made by the scripted
  shot list (default Steel theme, fixed window size, 2x, CC0 scenes), and
  are remade each release. [OP]
- **UIC-305** **Clean room.** Guide pages are written only from Operandi's
  own app, code and behaviour-only specs. They never name, quote,
  paraphrase or show other applications or their makers. [OP]

Added 2026-11-23. *Applies from: when the guide skeleton merges.*

---

## Rule index

1 General UIC-01..07 · 2 Menus/layouts UIC-10..19 · 3 Tabs/buttons
UIC-20..27 · 4 Lists/trees UIC-30..42 · 5 Fields UIC-50..59 · 6 Popovers
UIC-60..66 · 7 Viewports/windows UIC-70..78 · 8 Dialogs UIC-80..99 · 9 Property
panels UIC-100..126 · 10 Tree lists UIC-130..157 · 11 Commands UIC-160..165 · 12 Rendering UIC-170..174 · 13 Popovers vs floating windows UIC-180..188 · 14 Layouts UIC-190..219 · 15 Command history and macros UIC-220..239 · 16 Future doors UIC-240..259 · 18 User Guide UIC-300..305. Gaps in numbering are left for
later rules in the same topic; never renumber an existing rule.
