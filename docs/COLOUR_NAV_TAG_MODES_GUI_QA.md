# Colour field, Colour Picker and Materials mode: GUI checks

Branch `colour-nav-tag-modes`. One pass, any scene with a material, a light
and a mesh with two materials.

## Colour field (note 07)

1. CN-01: Shading > a Material > Diffuse Color. Drag the middle (G) number
   right about 100 px: G rises ~0.5, R and B stay, the viewport/preview
   follows live. Cmd+Z once restores the start colour.
2. CN-02: set (0.8, 0.4, 0.2); Shift-drag right: all three rise together
   (4:2:1), hue unchanged.
3. CN-03: pure red; Ctrl-drag right ~720 px: hue goes round once and back
   to red. Ctrl-click doesn't open a context menu.
4. CN-04: (0.8, 0.4, 0.2); Alt-drag far left: becomes (0.8, 0.8, 0.8).
5. CN-05: click without moving: the picker opens under the field. Move the
   pointer away: it closes; the colour kept. Esc also closes it.
6. Click on a number (not just the background): the picker opens at once
   on release. A double-click doesn't turn a number into a text field;
   numbers change only by dragging or in the picker's fields.
7. Right-click: Copy Color, Paste Color on another field, Reset to Default
   (theme colours in Preferences).
8. CN-06: Preferences > Input > Color Unit System Integer: numbers 0–255;
   Hexadecimal: three hex pairs; drags feel the same speed. Same setting in
   the picker's Options tab.
9. CN-07: picker Stops 2.0, click the right end of the value strip: RGB
   (HDR) shows value 4.
10. CN-08: select three materials in the Shader Tree, Ctrl-drag a Diffuse
    Color swatch: all three turn hue by the same amount; one undo step.
11. Picker: Color Model HSV / Kelvin / RGB, Hue as Wheel off (square +
    Axis), Dynamic Color Strip, RYB wheel; numbers update live.
12. Light colour, volumetric colour, wireframe colour, camera overlay
    colours, Set Material colour and Preferences theme colours all use the
    same field.

## Materials mode (note 04)

1. Modes bar: after Polygons, a divider, "Materials" and a popup showing
   "Material". Smoothing Group is greyed with "no smoothing group tags
   yet".
2. Press 4 (or the button): click a polygon: every polygon of that
   material is selected, drawn as an outline (not filled). Polygons button
   is not lit.
3. Shift-click adds a region, Ctrl-click removes one, click empty clears;
   lasso selects whole regions; each is one undo step.
4. The material's group is selected in the Shader Tree.
5. Popup to Part / Selection Set: clicks select by that tag instead.
6. With regions selected, Extrude works on them.
7. Click Materials again: back to Items. Alt+click Materials in Polygon
   mode: the selection grows to its whole regions.
