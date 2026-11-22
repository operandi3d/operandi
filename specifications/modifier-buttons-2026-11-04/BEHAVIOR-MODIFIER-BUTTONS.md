# Modifier-key button variants: behaviour spec

Status: v1.5, 2026-11-04 (MB-Q1 to Q5 answered, MB-Q6/Q7 decided by Jason). Behaviour only. Sources: Jason's screen recording (64 s, Polygon toolbox
open, Shift, Ctrl, Option and Cmd each held over the viewport), Modo's plain-text form config,
public docs (silent on this feature). All open questions are settled; see QUESTIONS.md.

## 1. What the user sees

A toolbar or toolbox button can carry up to one *variant* per modifier. While that modifier is
held, the button turns into the variant: new label, new action, and the variant's own on/off state.
Let go and it turns back.

### 1.1 Observed in the recording

| Button (base label) | Shift | Ctrl | Option (Alt) | Cmd |
|---|---|---|---|---|
| Vertices (selection mode) | - | - | **Convert** (convert current selection to vertices) | - |
| Edges (selection mode) | **Bounds** (select boundary edges of the selection) | **Boundary** (a second boundary-select variant) | **Convert** (to edges) | - |
| Polygons (selection mode) | - | - | **Convert** (to polygons) | - |
| Mesh Constraint | - | - | **Options** (opens its options popover) | - |
| Symmetry | - | - | **Options** | - |
| Snapping | - | - | **Options** | - |
| Work Plane | - | - | **Options** | - |
| Render (shortcut F9) | - | - | **Render Window** (open the last render), shown truncated as "Render ..." | - |
| Make (toolbox, shortcut P) | - | - | **Make Polygon...** (same command, but with its dialog) | - |

"-" means no change. No Polygon-tab tool button (Bevel, Extrude, ...) changed under any modifier.
Cmd changed nothing anywhere.

### 1.2 Also in the defaults (from config, not seen on screen)

| Button | Shift | Ctrl | Alt |
|---|---|---|---|
| Cube / Sphere / Cylinder / Cone / Toroid (Create) | **Cube Item** etc. (adds a ready-made procedural item) | **Unit Cube** etc. (adds a 1 m primitive at the origin, no drag) | - |
| Set Material... | - | **Set Smoothing Group...** | **Set Part...** |
| Preview | - | Preview (as a popover instead of a window) | - |
| Panel-opening buttons (many) | - | - | "Open in a Popover" / same panel as a pop-over |
| Animation key buttons | - | - | **Insert** (key with insert) |

Scale of the feature in the shipped defaults: about 670 variants on about 400 buttons. Alt is the most
common (about 280), then Ctrl (about 210), then Shift (about 175). One button uses a two-key
combination (Alt+Shift). Patterns per button: Alt only is the most common; Shift+Ctrl+Alt all
three is second; then Ctrl+Shift, Ctrl only, Shift only.

### 1.3 Live update

- The label swaps the moment the key goes down and swaps back the moment it comes up. It does not
  wait for mouse movement (in the recording the label changed about half a second *before* the status
  bar hints did, and the status bar only catches up on mouse move).
- The mouse was over the viewport, not the button, so the swap is global: every visible button
  updates, not just the hovered one.
- The swap also happens while a text field (such as the Command field) has keyboard focus.
  (Confirmed by Jason, MB-Q5.)
- Several key presses in a row (Shift, release, Ctrl, release, Option, release) each swapped and
  restored cleanly, with no flicker or leftover state.

### 1.4 Visual cues

- The only cue is the label (and the action behind it). No colour, badge, outline or icon marker
  says "this button has a variant", before or during the hold.
- The button icon stays the same.
- The keyboard-shortcut hint on the right of the button (P, F9) disappears while the variant is
  shown, because the shortcut belongs to the base action.
- Highlight state follows the *variant*: the lit "Polygons" mode button went unlit under Option,
  because "Convert" has no on/off state. The selection-mode icon at the top of the toolbox also went
  unlit, and one viewport-header icon changed its glyph, so header icon buttons follow the same rule.
- The button keeps its width. A longer variant label is cut with an ellipsis ("Render Window" became
  "Render ...").

### 1.5 Click behaviour

- Clicking while the modifier is held runs the variant instead of the base action: whatever the
  button shows is what runs. (Confirmed by Jason, MB-Q1.)
- The action is fixed at **mouse press**. Changing the modifier before mouse release makes no
  difference. (Confirmed by Jason, MB-Q2.)
- Holding two modifiers when the button has no variant for that exact pair: the keys cancel out and
  the button shows its normal (base) label and action. (Confirmed by Jason, MB-Q3.) A button can
  still define a variant for a pair, such as Alt+Shift, and then that pair shows it.
- An open tooltip switches to the variant's tooltip when the key goes down. (Confirmed by Jason,
  MB-Q4.)

### 1.6 Cmd on macOS

Modo's variant system knows only Shift, Ctrl and Alt (Option). Cmd is not a qualifier at all, so
holding Cmd changes no button. On a Mac, "Ctrl" here means the physical Control key.

### 1.7 Not part of this feature

The status bar's mouse hints ("Shift-Left Click: Selection: Range", "Alt-Left Click and Drag:
Navigation: Rotate") also react to modifiers, but that is the input-map hint system, a separate
feature. Out of scope here.

## 2. Proposed mechanism for Operandi

One rule: **a button is a list of (modifier set → action) entries, resolved against the live
modifier state.** No per-button code.

### 2.1 Data

```text
Button {
  id
  base:     { action, label, icon?, tooltip? }
  variants: [ { mods: Shift | Ctrl | Alt | Shift+Alt | ..., action, label?, icon?, tooltip? } ]
}
```

- `label` missing on a variant means keep the base label. (Modo does this for about 20 of its
  variants, mostly ones that just add a dialog.)
- `icon` missing means keep the base icon (Modo never changes it on the toolbox buttons).
- Shortcut hint, enabled/disabled state and lit/unlit state are never stored. They come from the
  resolved action, so they stay correct for free.

### 2.2 Resolution

1. Keep one app-wide `held_mods` value, updated on key down/up *and* on window focus regain (keys
   pressed while another app was focused must not leave a stuck variant).
2. For each visible button: pick the variant whose `mods` equals `held_mods` exactly. If none,
   show the base. (Matches Modo, MB-Q3: extra keys cancel out to the base.)
3. Cmd is ignored for matching on macOS, as in Modo, so Cmd shortcuts never flip buttons.
   **DECIDED (Jason 2026-11-04, MB-Q6).**
4. A repaint is only needed when `held_mods` changes, so the cost is near zero.

### 2.3 Click

- Resolve at **mouse press** and lock that choice until release. What you saw when you pressed is
  what runs, even if a finger slips off the key mid-click. (Matches Modo, MB-Q2.)
- The click must not pass the modifier on as a second meaning. Shift-click on a variant button runs
  the variant, not "the base action with Shift".

### 2.4 Where the "intelligent" part lives: rule templates

Most of Modo's variants follow a few patterns, so Operandi should state the pattern once and apply it
by category. Each button can still override or opt out.

| Template | Applies to | Variant |
|---|---|---|
| `options-popover` | any toggle that has an options panel (Snapping, Symmetry, Work Plane, Falloff, Action Center, Mesh Constraint) | Alt → "Options", opens its panel |
| `convert-selection` | selection-mode buttons | Alt → "Convert", converts the current selection to that mode |
| `primitive-quick` | primitive buttons | Shift → "{Name} Item", Ctrl → "Unit {Name}" |
| `with-dialog` | instant commands that also have a dialog (Make) | Alt → "{Label}...", same command with dialog |
| `open-as-popover` | buttons that open a panel/window | Alt (Ctrl for Preview) → same panel as a pop-over |

Templates are plain data with `{Name}`/`{Label}` filled from the button. A template adds nothing
where its target action does not exist, so it can be applied broadly.

### 2.5 Operandi additions (not Modo behaviour)

- **Discoverability. DECIDED (Jason 2026-11-04, MB-Q7).** Modo gives no hint that a button has
  variants. In Operandi the tooltip lists them in one line, e.g. "Shift: Bounds · Ctrl: Boundary · Alt: Convert". No new visual clutter.
- **Editable.** Store the button table in the same user-editable config as keybindings, so the
  planned Input Editor can show and change variants too.
- **Ellipsis.** Keep button width fixed and cut long variant labels, as Modo does, so the toolbar
  never reflows while a key is held.

## 3. Acceptance cases

| # | Do | Expect |
|---|---|---|
| MB-01 | Hold Alt with the mouse over the viewport | Vertices/Edges/Polygons read "Convert"; Snapping/Symmetry/Work Plane/Mesh Constraint read "Options"; Render reads "Render ..." |
| MB-02 | Release Alt | All labels, shortcut hints and lit states return at once |
| MB-03 | Hold Shift | Only Edges changes, to "Bounds" |
| MB-04 | Hold Ctrl | Only Edges changes, to "Boundary" |
| MB-05 | Hold Cmd (macOS) | Nothing changes |
| MB-06 | In Polygons mode, hold Alt | Polygons button goes unlit while it reads "Convert" |
| MB-07 | Hold Alt on the Make button (shortcut P) | Reads "Make Polygon...", shortcut hint hidden |
| MB-08 | Alt-click Snapping | Options panel opens; snapping on/off unchanged |
| MB-09 | Hold Alt, switch to another app, release Alt there, come back | Buttons show base labels |
| MB-10 | Press button with Alt, release Alt before mouse-up | Runs the variant |
| MB-11 | Polygon-tab tools (Bevel, Extrude, ...) under any modifier | No change |
| MB-12 | Press button with no key, press Alt before mouse-up | Runs the base action |
| MB-13 | Hold Shift+Alt over Edges (has Shift and Alt variants, no Shift+Alt variant) | Shows "Edges" |
| MB-14 | Hover Edges until its tooltip shows, then hold Shift | Tooltip changes to the "Bounds" one, live |
| MB-15 | Click into a text field, hold Alt | Buttons still swap |
