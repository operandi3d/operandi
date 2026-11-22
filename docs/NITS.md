# Nits

Small bugs and issues Jason drops in the "Collect nits to pick" thread. Fixed in
batches, one branch per batch.

## Batch 1 (branch `nits-batch-1`)

- **N01** Bend: Action Axis choice (Auto/X/Y/Z) for the axis the bend turns around.
  Fixed: new field; provisional BD-Q7.
- **N02** Edge loop selection says "edge loop stops here" on a loop around an
  opening whose inner polygons are split.
  Fixed (assumed cause: hole corners with two spokes); LOOP-HOLE-01.
- **N03** Dragging the mini properties panel while making a primitive also drags
  the primitive.
  Fixed: presses on in-window pop-overs no longer reach the viewport.
- **N04** Make Polygon fails with "Nonmanifold edge or inconsistent winding" on
  a gap (symmetry was on).
  Fixed: symmetric picks interleave pick/mirror; Make now builds both
  polygons. SYM-MAKE-Q1.
- **N05** Edge Bevel refuses an open strip whose ends stop mid-surface.
  Fixed: end-slide refusal now 45° between strip and slide polygons
  (was 20° across the whole fan). EBC-Q11 update.
- **N06** Option+click on the Snapping button doesn't open Snapping Options
  (F11 does).
  Fixed: the opening click was read as a click outside the popover in the
  same frame and closed it at once.
- **N07** Mini Properties had its own title bar with an X instead of the
  popover header. Fixed: standard header (pin circle, no title); it opens
  pinned, so the circle closes it and the strip drags it.

## Batch 2 (branch `nits-batch-2`)

- **N08** Edge Bevel still refused open strips ending at sharp folds (a
  face's centre line from brow to lip). Fixed: such ends keep their point
  and the strip's two new edges close onto it.

## Batch 3 (branch `nits-batch-3`)

- **N09** Several selected items showed only one as selected in the
  viewport. Fixed: Item mode highlights every selected item.
- **N10** Delete removed only one of the selected items. Fixed (viewport,
  item list and the `delete` command).
- **N11** Ctrl+G groups: provisional, parents the selected items to a new
  Group Locator (the existing Parent to Group Locator command).
- **N12** Return didn't confirm Set Material. Fixed: a text field merely
  focused on opening no longer swallows the first Return (UIC-85).
- **N13** Wireframe lines too thick. Fixed: plain wire is one device pixel
  on Retina (1.5 in wireframe views).
- **N14** Lists hard to read. Shader tree % column drops the per-row "%".
  The lighter background and selection style wait for the default colour
  scheme rework; type icons instead of letter badges need generator icons.
- **N15** Editor colours stronger in tree lists: 32% row tint (was 15%),
  kept on selected rows, wider colour bar.

## Batch 5 (branch `nits-batch-5`)

- **N28** Bridge with Symmetry refused or paired wrong (symmetric picking
  selects four runs). Fixed: each side's pair is bridged by itself, the far
  side's Twist reversed; pairs on or across the plane once.
- **N29** Bevel refused Jason's polygons: "can't keep some of this mesh's
  map data" (a named polygon set, R-Ear), then "a UV outline folds over
  itself". Fixed: named selection sets ride along (indices kept); where a
  UV outline folds, branches or has a zero-length edge, that corner's UV
  stays put instead of refusing the Bevel (BV-Q04, provisional).
- **N30** With Symmetry the handle (Local especially) leaned to one side.
  Fixed: the picked side keeps whole connected pieces; a piece across the
  plane keeps its own centre, only pieces wholly on the far side drop out.
- **N31** The Action Center button shows the centre in use and is lit when it isn't None.
