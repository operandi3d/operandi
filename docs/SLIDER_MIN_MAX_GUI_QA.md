# Slider min/max labels: GUI check

Branch `ui-hints-minmax`. Use the QA app from `~/Developer/Operandi-Builds`
with its own data folder (`OPERANDI_DEV_DATA`).

1. Fresh data folder: Preferences > Defaults > UI shows **Slider Min/Max
   Labels** unticked. Panels look exactly as before (no bars, no labels,
   same row spacing).
2. Tick it. Preferences > Defaults > UI > Popup Transparency now has a thin
   bar along its bottom edge and "0 %" / "90 %" in small grey text under
   its left and right ends; that row is a little taller. Drag its arrows:
   the bar's filled part follows the value.
3. Rows with no upper or lower limit (e.g. an item's Position X/Y/Z) have
   no bar and no labels, and keep their height.
4. Open a few panels with limited values and check the labels read right
   and nothing overlaps: a light's Temperature (Kelvin), the Shader tree's
   noise Frequencies, an item's Opacity, a Subdivide dialog's numbers,
   Tool Properties of a primitive with a whole-number count (Sides,
   Segments).
5. The colour picker's channel fields and the image viewer's zoom
   (MINMAX-Q02): do the labels overlap the next row? Note which.
6. Greyed fields: labels and bar show greyed.
7. Untick it: every row returns to its old height at once.
8. Quit and reopen: the setting is kept. `pref.value ui.slider_min_max_labels`
   over MCP reads and sets it.
