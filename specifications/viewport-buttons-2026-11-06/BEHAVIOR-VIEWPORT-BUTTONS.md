# Viewport header buttons: behaviour (summary of spec v1 with v1.1 changes)

**This is a summary, not the full spec.** It was relayed to the build side
by the project's coordinator on 2026-11-06 and records what was built from.
Layout of the Properties popover: LAYOUT-PROPERTIES-PANEL.md (v1.2).

## Header row
Right end of each 3D view's header, left to right: Pan, Rotate, Zoom,
Maximise/Restore, Properties gear. Icons about 14 px, monochrome in the
header icon colour; hover brightens and shows a tooltip; highlighted while
dragging. Tooltips: "Pan view (drag)", "Rotate view (drag)", "Zoom view
(drag)", "Maximise viewport (0)" / "Restore viewport layout (0)",
"Viewport properties (O)". When the header is narrow these keep priority
over the left-side controls.

## Maximise / Restore
- In a view group (quad, tri) every view shows a maximise glyph. Clicking
  it on view X shows only X filling the group's frame; X's button turns the
  accent colour. Clicking again restores the group exactly, split positions
  included.
- It is the same view enlarged: view type, camera, display style and all
  properties kept; changes made while maximised persist after restore.
  Hidden views keep their state and just don't draw.
- v1.1: world scale is kept: the same units per pixel before and after;
  geometry stays the same size on screen and you see more around it. No
  re-fit.
- Numpad 0 and Alt+0 toggle the view under the pointer; rebindable.
- A view alone in its layout: no button (VB-Q1).
- The maximised state and which view save with the workspace layout and
  survive relaunch. One maximised view per group; groups independent.
  Instant, no animation.

## Pan, Rotate, Zoom
- Press on the icon and drag; the view changes live; release ends; Esc
  during the drag restores the view as at the press. Plain click and
  right-click (no drag) do nothing, no menu (VB-Q2, v1.1). Middle and
  right drags act like left drags; modifiers ignored (VB-Q3). Not an undo
  step.
- v1.1: the pointer stays locked on the icon during the drag (hidden,
  pinned), so the drag never runs out of screen; on release it is still on
  the icon.
- Rotate only in views that can orbit (Perspective, Camera, Light); the six
  axis views omit it, and the row updates when the view type changes.
  Rotate's rate equals the in-view orbit gesture.
- v1.1 Pan: the button moves the view in the drag direction, so the scene
  slides the opposite way (drag right: scene moves left; drag down: scene
  moves up), at about 3x the pointer's travel. Preference "Reverse Pan
  Widget Direction" (default off) flips it to grab feel, button only.
- Zoom: about the view centre. Preference "Zoom Drag Axis" (Left to Right,
  Right to Left, Bottom to Top, Top to Bottom) picks the direction that
  zooms in; default Left to Right (VB-Q5); only movement along that axis
  counts (horizontal only with the default).

## Properties gear
- Click opens "Viewport Properties" for that view, anchored under the gear;
  O opens it for the view under the pointer. Click outside, Esc, or the
  gear again closes it. Edits apply live, per view, saved with the layout.
- Vertical tabs on the right edge: Drawing and Control, Visibility,
  Advanced Options, Active Meshes, Inactive Meshes; collapsible sections; a
  revert-to-default for the view and a one tab / all tabs switch.
- Settings that duplicate existing controls (header view type and display
  style, Display menu toggles, Advanced Show Shadows) are the same
  setting, not copies.

## Open questions
VB-Q1 hide maximise in a single view; VB-Q2 plain click does nothing;
VB-Q3 all mouse buttons the same, no modifiers; VB-Q4 whether ortho views
are linked (not built; awaiting a decision); VB-Q5 Zoom Drag Axis default
Left to Right.
