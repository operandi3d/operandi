# HDRI environment: GUI checklist

Branch `hdri-environment`. Use the QA build in `~/Developer/Operandi-Builds`
with its own data folder; test HDRIs are in
`~/Developer/Operandi-Content/Test-Library/PolyHaven/HDRIs`. Mark each line
pass/fail with a note.

## Properties (stage 1)

1. New scene, Shader Tree > Environments > select Environment. The
   Environment tab shows Intensity, then an **Environment Image** section
   (Image "(none)", Load Image…, Clear Image, Rotation "0.0 °"), then
   **Environment Visibility**. Clear Image and Rotation are greyed, not
   hidden; hovering them says why.
2. Load Image… opens a file panel offering HDR, EXR, PNG, JPEG and TIFF.
   Pick an .hdr: Image shows its name; Edit > Undo removes it in one step.
3. Drag Rotation: the view's sky turns while dragging; one Undo puts it
   back. Type 90: the picture's centre (what was straight ahead along +Z)
   is now to the right (+X). Render (F9 / Preview): it turned the same way.
4. Load Image… with another file: the picture changes, Rotation and the
   layer's Opacity stay.
5. Clear Image removes the image layer (one Undo brings it back).
6. Select the image layer under the environment: its tab has Image,
   Load Image… and Rotation; the tooltip no longer mentions clip.addStill.

## Viewport background (stage 2)

7. Viewport Properties > Background: Environment. The HDRI fills the
   background behind the grid and meshes, in Perspective; orbiting turns it.
8. Top / Front / Right views show one flat colour (the direction they look
   along), as an orthographic render does.
9. Environment Intensity 2: the picture brightens; layer Opacity 50 %: the
   gradient shows through. Turn the layer's Enable off: gradient only.
10. A big (8k) HDRI: the UI stays responsive while it loads (gradient
    first, then the picture).

## Viewport lighting (stage 3)

11. Advanced style with an HDRI: meshes take the picture's colours (a
    sunset HDRI tints them warm), sides facing bright parts are brighter.
    Rotation turns the lighting with the sky. A metallic material reflects
    a blurred sky.
12. Clear Image: Advanced goes back to its usual flat ambient.

## Drops (stage 4; UIC-74 deviation, Jason OK 2026-11-12)

13. Drag an .hdr or .exr from Finder onto a 3D view's empty space: it
    becomes the top environment's image (one Undo), no Backdrop.
14. The same file dropped onto a mesh: a Backdrop, as before. A .png or
    .jpg on empty space: a Backdrop, as before.
15. Preset Browser: drag an HDRI onto a view's empty space, or
    double-click it: it becomes the environment image.

## Render (stage 5)

16. Load a sunny HDRI (a visible sun), Global Illumination on, render a
    white sphere on a plane: the sun's shadow is clear and the picture has
    little noise at 64 samples (the world is importance sampled).

## Environment presets (stage 6)

17. File > Save Environment Preset…: a dialog with Include Lights,
    Include Selected Meshes (greyed with no mesh selected; hover says why),
    Include Cameras (unticked) and Copy Images Next To Preset. OK opens a save panel for
    a .openv file. Right-clicking an environment or its Environment
    Material in the Shader Tree offers the same.
18. The Preset Browser lists the .openv as an Environment Preset with a
    ball thumbnail lit by it.
19. New scene, double-click the preset (say "Studio.openv"): the default
    environment is gone, the preset's environments (same order, names,
    Intensity) are in, its light is inside one item group "Studio" (its
    materials under a Shader Tree group "Studio"); the new top environment
    is selected in the Shader Tree; one Undo puts it back. Hovering the
    group shows "From preset: Studio".
20. Rename the "Studio" group, add a spot light by hand, then right-click >
    Load another preset: the renamed group and its light go, the new
    preset's group comes in, the hand-made spot stays. Applying the same
    preset twice leaves one copy.
21. Drag a preset onto a 3D view: a small Replace / Add menu; Add puts its
    environments on top (they show). Drag onto the Preview: it adds.
22. With Global Illumination off in the Preview, applying a preset shows
    "Turn on Global Illumination so the environment lights the scene"
    once per session.
23. Move a preset saved with Copy Images to another folder: it loads with
    its picture. Delete the picture of one saved without copying: it loads,
    the picture shows missing (magenta), one message lists it.
