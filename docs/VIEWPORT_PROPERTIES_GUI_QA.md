# Viewport Properties phase 1 — GUI checklist

Branch `viewport-props`. Use the QA app with its own data folder
(`OPERANDI_DEV_DATA`). Start from a scene with two meshes and the views
split into Top (left) and Perspective (right): View > Split View, then set
the left view to Top.

1. **O opens near the pointer.** Pointer over Top, press O. The panel
   opens with its top-left just above-left of the pointer, titled
   "Viewport Properties: Top"; Top gets a thin orange outline. Near a
   screen corner it is nudged fully on screen.
2. **Leaving closes it.** Move the pointer just past the panel's edge
   (under 8 px): it stays. Move further out: it closes with no click.
   Esc also closes it. Open a dropdown or a colour field's picker that
   hangs outside the panel and move onto it: the panel stays.
3. **Gear and menu.** The header gear opens it hanging under the gear;
   the gear again closes it. View > Viewport Properties opens it under the
   active view's gear. With Show Viewport Controls and Information off, O
   still opens it.
4. **Pin.** Click the hollow circle at the title bar's right: it fills
   orange and the panel stays when the pointer leaves. Drag the title bar:
   the panel moves. Pick the Inactive Meshes tab. Quit and relaunch: the
   panel is back, pinned, same place, same tab. Click the filled circle:
   it closes.
5. **Target.** Pinned to Top, move across Perspective and change a row:
   Top changes, Perspective doesn't. Press O over Perspective: the title
   reads "Viewport Properties: Perspective" and the outline moves; the
   panel stays where it is. Close the target view (View > Close View):
   the panel closes even pinned.
6. **Header shading selector.** The Display button's menu lists only the
   ten styles (then the view toggles and presets) — no Active/Inactive
   submenus, no Same as Active / Invisible, no Alt-click. Picking a style
   sets Active Meshes > Shading Style in the panel and the button label.
7. **Inactive style.** Select one mesh. Inactive Meshes > Shading Style
   Wireframe: only the unselected mesh turns wireframe, only in this view.
   Deselect all: the mesh selected last keeps the active look, the other
   stays inactive (VP-04/VP-T8). Select both (Item List Shift-click): both
   draw active.
8. **Inactive wire colour.** The unselected mesh's plain wire is black by
   default. Set Inactive Meshes > Wireframe Color red in Top only:
   Perspective stays black. Locked, seam and selection edges keep their
   own colours.
9. **Overrides.** Make Inactive Same as Active: the unselected mesh draws
   like the selected one (its wire colour too) and the rest of the tab
   greys in place. Make Inactive Invisible: the unselected mesh disappears
   from this view only (still in the Item List, still renders). With both
   on, Invisible wins.
10. **Theme colour.** Preferences > Display > Colors > Inactive Wireframe
    to blue: views whose inactive colour was never edited turn blue; the
    view set red in step 8 stays red.
11. **Split copies.** Split a view after changing its panel settings: the
    new view starts with the same settings, then each changes on its own.
12. **Rows.** Wireframe Overlay and Opacity grey when Shading Style is
    Wireframe; Opacity also greys with Overlay None. Opacity's arrows at
    the right end drag the value. Smooth Shade off draws flat polygons in
    this view only.
13. **Last selected mesh (VP-04).** Select mesh A in Polygon mode, click
    empty space in Item mode to deselect all, switch back to Polygon mode:
    A still looks active and Select All (Ctrl+A) selects A's polygons.
    Open a saved scene fresh: every mesh draws inactive until one is picked.
14. **Visibility tab (P2).** The work plane is off by default: only the
    floor grid shows, even after moving the work plane. Turn on Show Work
    Plane (Work Plane Settings > Display, or Preferences > Display): the
    work plane's blue grid shows at rest too, and stays on after a
    restart. With Show Grid Inactive Work Plane (the default), moving the
    work plane then hides the floor grid; Show Work Plane Active in this
    tab shows the work plane only once it is moved. Draw Faded Grid off:
    far grid lines no longer fade. Faded Grid Color Axes off: the floor's
    axis lines draw plain. Show Locators off hides group locators; Show
    Texture Locators on draws every texture gizmo.
15. **Selections.** Polygon mode with polygons selected: Show Selection
    Normals draws dashed lines out of them; Show Selected Filling off
    removes the tint; Show Selected Outline off removes the outline; Show
    Selections off hides all of it and greys those rows plus Rollovers.
16. **Scene lighting.** A scene light and the Advanced style: Show Scene
    Lighting off lights the view from the view's own light; on, from the
    scene's light (with its shadow).
17. **Drawing and Control (P3).** With two cameras, Camera Item lists
    (Current Selection) and both; picking the unselected one switches the
    view to look through it (header names it). Mouse Control > Orbit
    Selected on: select a mesh off to the side and orbit; it stays put on
    screen while the view turns around it. Trackball Rotation now sits
    here, not on Visibility.
18. **Presets (P4).** Presets ▾ lists the saved viewport presets (the
    Display menu's) above Save Preset… and Revert to Default; picking one
    applies it, picking it again takes it off. One Tab / All Tabs say
    "3D Viewport".
19. **Advanced Options (P5).** Advanced style with a light: Ambient
    Intensity 20 % darkens the shadowed sides, 100 % brightens them;
    Ambient Color red tints them. Other styles don't change.
20. **Point size (P5).** Active Meshes > Independent Point Size on, Point
    Size 10: this view's points grow; other views keep Preferences' size.
    Point Size greys while Independent Point Size is off.
21. **Transparency Overrides (P5b).** Active Meshes > Transparency Mode
    Ghost: the selected mesh turns see-through (faint front, greyer back)
    and its hidden edges show; X-Ray: only the far side shows. Overlay on:
    it draws over meshes in front of it. Flat Shaded: no lighting. The
    rows grey while the mode is Normal or the style is Wireframe; the
    Inactive tab has the same rows (no Overlay).
