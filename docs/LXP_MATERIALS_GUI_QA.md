# Modo material presets (.lxp): GUI QA

Branch `lxp-materials`. Spec: `specifications/lxp-materials-2026-11-15/`.
Use the QA app with its own data folder; add the Modo content folder as a
browser location. Nothing is written into the content folder.

1. Materials filter: material .lxp files show as normal tiles with Modo's
   own thumbnail and the kind "Modo material preset" (LXP-01). City Scape 01
   (no material) stays greyed; its tooltip says "Modo preset with no
   material we can read." (LXP-04).
2. Tiles that lost layers show an accent dot top right; hovering it shows
   the note, clicking puts the note in the status bar (LXP-02). Brown
   Leather shows "partial" bottom left (LXP-05).
3. Polished Gold onto a sphere: metallic gold; Cycles render looks gold,
   its edge roughness follows the Incidence gradient. One Cmd+Z removes it.
4. glass_crown: clear glass (IOR 1.52) in Cycles.
5. White LED: glows near white, not burnt out: its Luminous Intensity
   reads 1.7 (LXP-B02b).
6. Unreal Bowling Ball and Unity Bowling Ball: image maps show in the
   viewport (Advanced) and render (base colour, normal, roughness/metal).
7. A gradient-on-Incidence material (Polished Gold) in the viewport: the
   roughness/colour change toward the silhouette, per pixel (stage A).
8. Double-click and right-click Load on a selected material group, and a
   drop on a polygon selection, behave like any library material (LXP-03).
9. A preset with a Diffuse Color gradient (any Enhanced modo material
   with a colour ramp): the ramp shows its colours, not grey (LXP-B07a).
10. A mesh light and a cylinder light at Intensity 1, Visible to Camera:
    each reads about as bright as White LED's surface, 1/π of before
    (LXP-B02a). Old scenes with these lights render darker.
