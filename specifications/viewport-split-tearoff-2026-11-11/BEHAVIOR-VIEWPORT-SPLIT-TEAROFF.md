# Viewport splitting and tear-off (v1, 2026-11-11)

Behaviour only, words only. IDs VST-xx. Builds on the panel corner dot spec
(PCD, 2026-11-06), which Operandi has already built; this spec pins down the
parts PCD left loose: drag direction, where the split lands, joining and
closing, minimum sizes, what a new view inherits, undo, and the torn-off
window itself.

Tags: **[doc]** the reference app's public user guide (paraphrased);
**[seen]** watched in the reference app (2026-11-06 session, recorded in the
PCD spec); **[cfg]** its plain-text command help or window definitions;
**[proposal]** not established for the reference app; a sane Operandi rule.
Bible rules are cited as UIC-nn.

Words used: a *panel* is any slot in a layout (a 3D view, a UV view, a list,
a properties panel). A *view* is a 3D or UV viewport panel. The *dot* is the
grip at a panel's top-left (UIC-71, PCD-01). A *split* is a row or column of
panels separated by draggable *dividers*.

## 1. Ctrl+drag on the dot: split in the drag's direction

**VST-01 Gesture.** Hold Ctrl, press on a panel's dot, move. Once the
pointer has moved past the drag threshold (the same few points that start a
plain dot drag, PCD-07), the gesture is a split. [doc] On Mac this is the
real Ctrl key, and Ctrl+press on the dot must never turn into a right-click
(bible key rule, Jason 2026-11-08). [proposal from the bible]

**VST-02 Direction.** The direction is the drag's dominant axis, measured
from the press point to the current pointer:
- mostly right: the new panel goes to the **right** (side by side);
- mostly left: the new panel goes to the **left**;
- mostly down: the new panel goes **below** (stacked);
- mostly up: the new panel goes **above**.
The reference app's guide says only "in the direction of the drag,
horizontally or vertically" [doc]. Jason, from use of the reference app:
dragging down creates the new panel below and dragging right creates it to
the right, "and so on" for the other directions. [Jason, 2026-11-11] Up and
left follow by symmetry and are what Operandi already does. A drag exactly on
the diagonal counts as sideways. [proposal]

**VST-03 Live direction.** The direction follows the pointer until release:
dragging down and then swinging right gives a right split. Only the state at
release counts. [proposal]

**VST-04 Feedback while dragging.** Draw a preview of the result on the
panel being split: an insert line (bible "insert line" colour, as UIC-72) at
the split position, and a translucent accent fill over the half the new panel
will take. No drop zones on other panels (Ctrl-drag never docks). The pointer
shows the same panel cursor as a plain dot drag. [proposal; the reference app
showed no highlight during a plain dot drag, PCD-07, and the split case was
not watched]

**VST-05 Where the split lands.** At half: the panel is divided into two
equal parts, exactly like the menu's Split Horizontal / Split Vertical. [doc
for the menu items: "evenly"; the drag's position not measured, PCD-Q3, still
open as VST-Q02]

**VST-06 Pointer leaves the panel.** The split still applies to the panel
whose dot was pressed, wherever the pointer is on release, even outside the
window. Ctrl-drag never tears off. [proposal]

**VST-07 Cancel.** Esc before release cancels: nothing changes. Releasing
inside the drag threshold (no real movement) does nothing beyond making the
panel active (PCD-06). [proposal, matching PCD-30's Esc]

**VST-08 Modifier timing.** Ctrl is read when the drag starts (when the
threshold is crossed). Pressing or releasing Ctrl later doesn't switch
between split, tear-off and copy. [proposal]

**VST-09 Lock.** With Lock Thumb Dragging on (PCD-12), Ctrl-drag does
nothing, like every other dot drag; the dot's menu still offers Split
Horizontal / Vertical / Quad. [doc: the lock turns off thumb drags]

**VST-10 Which panels split.** Any panel with a dot can be split: views and
also lists and properties panels. [doc: "split ... into two viewports of the
same type"; the guide's custom-layout walkthrough splits non-3D panels]
Where Operandi's side panels are not yet in the layout tree (PANEL-Q01 on
the Operandi side), Ctrl-drag on their dot does nothing until they are.
[proposal]

**VST-11 Nested direction.** Splitting a panel that already sits in a row
along the same axis (for example a right split of a panel inside a
side-by-side row) adds the new panel to that row next to the source, taking
half of the source's width; the other panels in the row keep their sizes.
Splitting across the row's axis turns the source's slot into a small column
(or row) of two. [proposal; the reference app's layouts are grids where
panels can span cells, but how a split is added to that grid wasn't
watched]

**VST-12 Active panel afterwards.** The new panel becomes the active one
(amber dot). [seen for Split Horizontal, PCD-17]

## 2. What the new panel inherits

**VST-13 Same type, same settings.** The new panel is a copy of the source:
same panel type and the same view settings at the moment of the split. For a
3D view that means the view type (Perspective, Top, Camera and so on), the
camera angle, position and zoom, shading style, display toggles, grid and
the visibility options. [seen, PCD-16; doc: "two viewports of the same type"]

**VST-14 Then independent.** After the split the two are separate: orbiting,
zooming or changing the type of one doesn't change the other. A Camera view
copied from a Camera view looks through the same scene camera; moving that
camera moves both, because it's the scene camera, not the view. [proposal;
the first half matches Operandi's existing check V-02]

**VST-15 Not copied.** Scene content, selection, the active tool and the
undo history are the scene's, not the view's: both panels show and edit the
same scene. The maximise state is not copied (the new panel starts
unmaximised). A user-set panel title, if any, is not copied; the new panel
gets the next default name (Operandi's "Viewport n"). [proposal]

**VST-16 Presets.** If the source was loaded from a saved view preset, the
copy refers to the same preset, so Revert on either restores that preset.
Later. [proposal]

**VST-17 UV views.** A UV view splits and copies the same way: the copy
shows the same UV map, the same UV-space framing and the same UV display
options, then pans and zooms on its own. See section 7. [proposal]

## 3. Joining and closing splits

**VST-18 There is no join gesture.** The reference app has no "merge two
panels into one" command or drag. A split is undone by deleting one of the
two panels; its neighbours grow to fill the space. [doc: the guide's advice
for an accidental split is to delete the extra pane with the dot's Delete]

**VST-19 Who fills the hole.** When a panel is deleted (dot menu Delete,
View > Close View, closing a tab in a tabbed panel's last slot):
- in a split of two, the other panel takes the whole slot;
- in a row or column of three or more, the space is shared by the two
  neighbours in proportion to their sizes (or all of it goes to the single
  neighbour when the deleted panel was at an end);
- a split left with one panel dissolves: that panel takes the split's place
  in its parent, so nesting doesn't pile up.
[proposal; the reference app's behaviour was "neighbours fill the space"
(PCD-07, seen), the exact share not measured, VST-Q03]

**VST-20 Dragging a divider never closes a panel.** Dividers stop at the
minimum size (section 4); you can't drag one panel shut. [proposal; the
reference app has separate collapsible areas (a thin line you click to open
or hide a region), which are fixed parts of its stock layouts and not a
general join; leave them out]

**VST-21 What can't be deleted.** The main window's last panel can't be
deleted, detached or torn off (PCD-33, built). A panel alone in a floating
window can't be deleted from its menu; closing the window removes it
(PCD-14, PCD-20). A maximised panel can't be deleted until it is restored
(PCD-14). [seen for the floating and maximised cases]

**VST-22 Dock instead of join.** To put a torn-off panel back, drag its dot
onto an edge of a docked panel (PCD-30 dock-beside, built). That, plus
Delete, covers every "join" case. [Operandi addition, approved 2026-11-06]

## 4. Minimum sizes

**VST-23 Docked panels.** Every docked panel has a minimum size. Suggested:
- a view (3D or UV): 120 x 90 points including its header;
- a list or properties panel: 100 points wide, and tall enough for its
  header plus two rows;
- a panel whose header is hidden (Min Header): 24 x 24.
[proposal; the reference app's docked minimums were not measured,
VST-Q04]

**VST-24 Split refused when too small.** A split (drag or menu) is refused
when either half would come out below the minimum along the split axis. The
split preview (VST-04) then draws in the theme's warning colour and release
does nothing. The menu items Split Horizontal / Vertical / Quad are greyed
for a panel that is too small to split that way. [proposal]

**VST-25 Dividers clamp.** Dragging a divider stops where either neighbour
(or, in a nested split, the smallest panel on that side) would go below its
minimum. [proposal]

**VST-26 Window resizes.** When the main window or a floating window shrinks,
panels keep their share of the space (dividers sit at fractions of the
split, not at fixed pixels), except panels with Lock Width / Lock Height on
(PCD menu items 13-14, later), which keep their size while the others give
way. If the window gets so small that the minimums can't all fit, panels
shrink below the minimum rather than overflow the window, and return to
their shares when it grows again. [cfg: the reference app's stock layouts
store divider positions as fractions; doc for the size locks; the
overflow rule is a proposal]

**VST-27 Floating windows.** A floating window can't be made smaller than
the minimum of what it holds, and never smaller than 200 x 160 points (the
reference app's own floating panel windows use minimums around 200 x 200;
Operandi's view windows already use 200 x 160). [cfg for the reference
range; Operandi value kept]

## 5. Tearing a view off into its own OS window

**VST-28 Gesture.** A plain drag on the dot, released outside every panel,
tears the panel off (PCD-07, built). The dot menu's Detach does the same
without a drag (PCD menu item 3). Shift+drag tears off a copy and leaves
the original (PCD-08, built). [doc][seen]

**VST-29 It is a real OS window.** The torn-off panel lives in its own
operating-system window, not a panel drawn inside the main window. It can
be moved to another monitor, sits in the OS window list, and is closed with
the OS close button. Operandi should open it through the same path as its
other floating panels (the shared floating-palette window helper, per
Jason's 2026-11-10 rule that floating panels float as their own OS window),
so there is one kind of floating window, not two. [doc: floating windows keep
the OS controls; project rule]

**VST-30 Window style.** The reference app has two floating styles: a
*Window* (full OS title bar with minimise, maximise and close) and a
*Palette* (a thin header with only a close control), and a global key
(backtick) shows or hides all palettes at once. [doc] For v1: torn-off views
open as Windows with full OS controls, and they stay on top of the main
window the way Operandi's palettes do. [DECIDED, Jason 2026-11-11, VST-Q05]
Palette style and the backtick toggle: later.

**VST-31 Where and how big.** On a drag tear-off, the window's top-left
corner lands at the release point and its content area is the panel's
current size. [seen, PCD-07] On Detach from the menu, the window opens over
the panel's old screen position at the panel's size. [proposal, PCD-Q4] In
both cases the window is pulled fully onto the screen the pointer is on, and
clamped to the minimum (VST-27). [proposal]

**VST-32 Title.** The window title is the panel's name ("Viewport 2", "UV",
"Items"). With more than one panel in the window, it names them all, like
the existing layout title rule. [seen: the reference app uses the panel's
preset name; Operandi already does this]

**VST-33 The hole.** The panel's old slot closes as in VST-19. [seen]

**VST-34 Inside the window.** The torn-off window is a full layout of its
own: its panels have dots, can be Ctrl-split, Shift-copied, maximised and
docked back, exactly as in the main window (UIC-71: "behaves like the main
window"). A panel alone in a floating window, dragged outside, opens a copy
in another window rather than leaving the first empty (PCD-10, built). [doc,
seen]

**VST-35 Same scene, same undo.** A torn-off view still shows and edits the
same scene; selection, tools and undo work across windows. Keyboard shortcuts
work in the floating window when it has focus, acting on the panel under the
pointer (UIC-70). [proposal; matches Operandi check V-04]

**VST-36 Closing.** Closing the window throws its panels away with no prompt
(PCD-20, built). Nothing in the scene changes. [seen]

**VST-37 Survives restarts.** Floating windows, their position, size and
contents, are saved with the layout and come back on the next launch
(UIC-73). If the saved position is off every connected screen, the window
reopens on the main screen near the main window. [UIC-73; the off-screen rule
is a proposal]

**VST-38 Min/max of the OS window.** OS maximise fills the screen with the
window's layout; OS minimise hides it. Neither changes the main window.
[proposal]

## 6. Undo

**VST-39 Layout changes are not undo steps.** Split, delete, tear-off, dock,
swap, divider drags, maximise and closing a floating window don't go into the
scene's undo history, and Undo never rearranges panels. A layout change in
the middle of modelling leaves the undo history exactly as it was. [DECIDED,
Jason 2026-11-11, VST-Q06; Operandi's layout already works this way, its
check L-05]

**VST-40 Recovering a layout.** The way back from an unwanted layout change is
Revert Layout (the stock layout, with confirmation) or a saved layout, not
Undo. [doc: the guide offers Revert Layout for a mistakenly modified layout]
An Operandi-only "Undo Layout Change" (one step, kept apart from scene
undo) is a possible later addition. [proposal, not for v1]

**VST-41 Scene dirty flag.** Layout changes don't mark the scene as
modified and are never saved into a scene file. [proposal]

## 7. Overlap with the UV viewport build

A separate Operandi build is adding a UV viewport now. Splitting and
tear-off must treat it as just another panel kind:
- the UV view gets a dot, the dot menu, Ctrl-split, Shift-copy, tear-off and
  docking with no special cases (VST-10, VST-17, VST-34);
- its copy inherits the UV map, framing and UV display options (VST-17);
- the reference app's UV layout is a 3D view split side by side with the UV
  view on the left [doc], so a stock "UV" layout is one split of the kind
  this spec defines;
- the reference app also opens the UV view as a floating palette from a
  toolbar button [doc]; that palette is an ordinary torn-off window holding a
  UV view (VST-29).
The two builds should agree on one thing early: the UV view must be a panel
kind in the same layout tree as 3D views (not a side panel outside it), so
splitting, deleting and tearing it off use the same code. Whoever lands
second adapts to the first.

## 8. Suggested v1 scope

Build: VST-02 four directions (check what exists), VST-03, VST-04 preview,
VST-06 to VST-08, VST-11, VST-13 to VST-15, VST-19 neighbour share, VST-20,
VST-23 to VST-25 minimums, VST-27, VST-29 shared OS-window path, VST-31 menu
Detach placement and on-screen clamp, VST-37 off-screen rule, VST-39 and
VST-41 (tests that layout changes leave undo and the dirty flag alone).

Later: VST-16 presets, VST-26 size locks, VST-30 palette style and the
backtick toggle, VST-40 layout undo.

Already built (keep, add tests where missing): VST-09, VST-12, VST-21,
VST-22, VST-28, VST-32 to VST-36.

## 9. Test cases (behavioural)

| Case | Do | Expect |
|---|---|---|
| T01 | Ctrl+drag a single view's dot 80 pt down, release | Two views stacked, each half the height; the new one is below and active; both show the same angle and zoom |
| T02 | Same, drag right / left / up | New view right / left / above |
| T03 | Ctrl+drag down, then swing right before release | Right split |
| T04 | Ctrl+drag, press Esc, release | No change |
| T05 | Ctrl+drag the dot onto the desktop | A split, not a tear-off |
| T06 | Ctrl+drag with Lock Thumb Dragging on | Nothing; menu Split still works |
| T07 | Ctrl+drag a view 150 pt wide to the right | Refused (halves below 120 pt); preview drawn in warning colour; Split Horizontal greyed |
| T08 | Three views in a row, sizes 200/300/500; delete the middle | Neighbours share the 300 in a 2:5 ratio: about 286 and 714 |
| T09 | Split then delete the new view | Back to one view at full size; no leftover nesting |
| T10 | Drag a divider toward a neighbour | Stops when that neighbour reaches its minimum |
| T11 | Make a modelling change, split, tear off, dock back, swap, then Undo | Undo removes the modelling change; the layout stays as it is |
| T12 | Split, then check the scene's modified flag | Unchanged |
| T13 | Tear off a view by drag | OS window, top-left at release point, same content size, titled with the view name; old slot closed |
| T14 | In the torn-off window, Ctrl+drag the view's dot down | The window now holds two stacked views |
| T15 | Quit with a floating window on a second monitor, unplug it, relaunch | Window reopens on the main screen near the main window |
| T16 | Split a UV view to the right | Two UV views of the same map and framing; panning one leaves the other |
| T17 | Rotate one of two split 3D views | The other doesn't move |
