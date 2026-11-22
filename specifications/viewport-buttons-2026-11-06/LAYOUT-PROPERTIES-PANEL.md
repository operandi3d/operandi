# Viewport Properties panel: layout addendum (spec v1.2, 2026-11-06)

Layout and control types for the gear popover from section 4 of BEHAVIOR-VIEWPORT-BUTTONS.md. This replaces any multi-column layout.

## 1. Overall shape
- One narrow column, fixed width about 420 logical px (including the tab strip). Height fits the content, capped at the window height minus a small margin, with a vertical scroll bar. The panel never flows into a second column and nothing overlaps.
- Title bar: "Viewport Properties", centred, with a small round close button at the far right.
- Second row, right-aligned, three small text buttons with drop arrows: Presets ▾ (Save Preset…, Revert to Default), One Tab ▾, All Tabs ▾. "One Tab" shows only the selected tab; "All Tabs" stacks every tab's sections in one long scrolling column, in tab order. The active mode is shown in the normal text colour, not as an orange pill.
- Tab strip: a narrow vertical strip (~22 px) on the right edge, full panel height. Tab names run rotated 90°, reading top to bottom. The active tab's text is the accent colour (orange); others are normal grey. Order and names: Drawing and Control, Visibility, Advanced Options, Active Meshes, Inactive Meshes. Use the full names.
- The content area sits left of the strip.

## 2. Sections
- Each tab holds collapsible sections. A section header is a full-width darker bar (~18 px tall) with a disclosure triangle at the left and the section name in bold-ish text. Clicking the bar folds the section. All sections start open. Collapse state is remembered per user.
- Sections follow each other directly in one column; no gap wider than one row between sections.

## 3. Rows
- Every setting is one row, ~20 px tall.
- Two-column form grid: label column ~38% of content width, labels right-aligned, small gap, then control column filling the rest up to a small right margin.
- Dropdowns (every multiple-choice setting) fill the whole control column: dark field, value left-aligned, small ▾ at right. Never use radio buttons or rows of toggle buttons in this panel.
- Checkboxes have no label in the label column. Box at the left edge of the control column, text to its right. Checked = filled with accent colour and a dark tick. Consecutive checkboxes form a tidy left-aligned stack.
- Numbers: a text field filling the control column, value left-aligned with unit ("30.0 %"), small drag/step arrow pair at the right end; drag on arrows to scrub. No sliders.
- Colours: one field filling the control column, colour as swatch background with R G B numbers on it in three equal parts; click opens colour picker.
- Disabled settings stay in place, greyed (e.g. Orientation when not orthographic).
- A blank half-row gap separates small groups inside a section (shown as "—").
- Settings not built yet are left out, not shown greyed.
- Tooltips on hover for every row.

## 4. Order and control type per tab ([D] dropdown, [C] checkbox, [N] number, [K] colour)
Drawing and Control
- Section View and Shading: View Type [D] (Perspective, Top, Bottom, Front, Back, Left, Right, Camera, Light); Orientation [D] (0/90/180/270°, greyed unless ortho) only if built; —; Overlay Drawing [C]; Show Viewport Controls and Information [C]; Show Wireframe in Item Mode [C]; —; Selected Item Drawing [D] (Wireframe, Filled, None); —; Background [D] (None, Gradient); —; Camera Item [D] ((Current Selection), scene cameras); Light Item [D] ((Current Selection), scene lights)
- Section Viewport Independence (only if built): Independent Center [C], Independent Scale [C], Independent Rotate [C], Master Viewport [D]
- Section Mouse Control: Trackball Rotation [D] (Default, Yes, No); Orbit Selected [C]
Visibility
- Item Visibility: Show Lights [C], Show Cameras [C], Show Locators [C], Show Meshes [C]
- General Visibility: Show Work Plane [D] (Always, Active, Never), Show Grid [D] (Always, Inactive Work Plane, Never), Draw Faded Grid [C], Faded Grid Color Axes [C], Show Backdrop [C]
- Selection Visibility: Show Selections [C], Show Selection Normals [C], Show Selected Filling [C], Show Selected Outline [C], Show Selection Rollovers [C]
- Performance: Mesh Smoothing [C], Use Shader Tree [C], Use Texture [C]
Advanced Options (Advanced display style only)
- Effects: Ambient Intensity [N %], Ambient Color [K], Show Shadows [C]
Active Meshes
- Shading: Shading Style [D] (display styles, menu order); Wireframe Overlay [D] (None, Uniform, Colored); Wireframe Opacity [N %]; Wireframe Color [K]; Smooth Shade [C]; Show UV Boundaries [C] (replaces any "Show UV Seam" row elsewhere); Show Geometry Boundaries [C]; —; Show Vertices [D] (Always, In Component Modes, In Vertex Mode, Never); Edge Thickness [N]; Show Cages [C]; Show Guides [C]
Inactive Meshes
- Overrides: Make Inactive Same as Active [C], Make Inactive Invisible [C]
- Shading: same rows/order as Active Meshes > Shading, for inactive meshes. Greyed when Same as Active or Invisible is on.

## 5. Remove from current build
- Multi-column flow, overlapping/wrapped labels.
- Button rows for View Type, Shading Style, Show Grid, Show Vertices, Background, Wireframe Overlay: all dropdowns.
- Sliders: become number fields.
- Any extra "show every vertex" row (Show Verts): Show Vertices = Always covers it.
- Short tab names: use full names.
- Settings on the wrong tab go where section 4 puts them.

## 6. QA
- One column ~420 px; nothing overlaps at any window size; long content scrolls.
- All multiple-choice settings are dropdowns; no radio, segmented buttons or sliders.
- Labels right-aligned in one column; checkboxes stacked at control column's left edge, text to the right.
- Tabs rotated on right edge with full names; All Tabs shows every section in tab order.
- Sections collapse and stay collapsed after closing and reopening the panel.
