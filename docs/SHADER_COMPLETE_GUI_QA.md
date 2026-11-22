# Shader tree complete — GUI check

QA build from branch `shader-tree-complete` ("Operandi shader-complete
<commit>.app" in ~/Developer/Operandi-Builds, with Cycles). "Render" means
File > Render Viewport (or the Preview window). Views in the Advanced
style unless a step says otherwise.

## Stage 1: texture locators (TL-Q30..Q32)

1. **Front projection.** Add a camera and a plane facing it. Give the plane
   an Image Map layer with any picture. In the layer's Texture Locator set
   Projection Type to Front. Projection Camera is now available (it's
   greyed for every other type) and shows "(none)". Render through that
   camera: the picture fills the camera's frame exactly, lined up with the
   frame's edges. The viewport shows the same mapping.
2. Move the camera: the picture slides across the plane with it (the
   plane's own UVs no longer matter).
3. Add a second camera and choose it as Projection Camera: the picture now
   projects from that camera, whichever camera renders.
4. Delete every camera: the panel says "No camera to project from: renders
   as UV Map", and the plane shows its UV mapping again.
5. **Light Probe.** On a sphere, set Projection Type to Light Probe with a
   mirror-ball photo: it wraps round the sphere the way the same photo
   looks as a Light Probe environment, in the viewport and in a render.
   Projection Axis changes which way is up.
6. **Implicit UV.** On a cube with a checker or grid image, set Implicit
   UV: every face shows the whole picture once, in the viewport and in a
   render, whatever the cube's UV map. Repeat 2 × 2 shows it four times
   per face.
7. Save, reopen: the projection types and Projection Camera are kept.
   Undo and redo step through each change.

## Stage 2: Advanced viewport

8. **Glass refraction.** A glass block (Diffuse 0 %, Transparent Amount
   100 %) turned about 35° in front of a red-and-green wall. With
   Refractive Index 1.6 (Use Refractive Index on), the red/green edge seen
   through the block is shifted against what's beside it. At 1.0 it lines
   up straight. Orbit: no flicker, and the wireframe and selection of
   things behind still show.
9. **Gradient environment reflections.** Add an Environment (4 Color
   Gradient) and a chrome ball (Metallic 100 %, Roughness 5 %): the ball
   reflects the gradient's sky and ground colours, and diffuse objects get
   the sky's colour from above and the ground's from below. Change a
   gradient colour: the reflections follow. With no environment, Advanced
   looks as before (flat ambient).
10. **Subsurface.** A sphere with Subsurface 100 % (red-scattering radius,
    the default) and the view light from the side: the shadow line is
    soft and reddish, not a hard dark edge. Subsurface 0 %: the hard edge
    is back.
11. **Coat tint.** Coat 100 %, Coat Tint red, base white: the surface
    looks red under a clear, white highlight.
12. **Coat bump.** A bumpy image on Bump, Coat 100 %, low coat roughness.
    Clearcoat Bump 100 %: the coat's highlight follows the bumps;
    0 %: the coat's highlight is smooth while the surface beneath is
    bumpy.
13. **Cut-out shadows.** A plane with a Stencil layer (a leaf picture or a
    half-grey constant), shadows on: its shadow is lighter by how much is
    cut away. It isn't leaf-shaped yet (VM-Q10).
14. **Procedural bump per pixel.** A Noise or Cellular layer on Bump:
    the Advanced viewport shows fine relief across each polygon (before,
    it only changed from corner to corner). A Checker on Diffuse Color
    has crisp squares inside large polygons in every style.
14b. **Displacement.** A smooth sphere, a Noise layer on Displacement,
    Displacement Distance 0.1 m: in Advanced the sphere's outline turns
    lumpy like the render. Drag a point with Move: the lumps follow
    smoothly while dragging. Other styles show the plain sphere.

## Stage 3: Edit Gradient window (GR-Q03 v2)

15. Add a Gradient layer (Diffuse Color). Under its colour strip, click
    **Edit Gradient…**: a window opens with Input Parameter, a large colour
    strip and the key list.
16. Presets ▾ > Rainbow: six colours spread over 0..90°; the viewport and
    a render show them. Undo puts the old curve back in one step.
17. Reverse: the colours run the other way. Distribute Evenly after
    dragging a key: the keys are evenly spaced again. Reset: one white key.
18. Presets ▾ > Recent lists the curves you replaced; picking one brings
    it back. Close the window and reopen: Recent still lists them.
19. Set the layer's effect to a scalar one (e.g. Roughness): the window
    shows the Value curve with Ramp Up / Ramp Down / Bell / Four Steps.

## Stage 4: Occlusion types (PQ-5) — render

20. A cube on a floor with an Occlusion texture on the floor's Diffuse
    Color. Render with Type Up: dark under overhangs. Concavity: dark
    in the creases where the cube meets the floor. Convexity on the
    cube: dark along its edges. Concavity & Convexity: creases light,
    edges dark, flat parts mid grey.

## Stage 5

21. **Shader Effect.** Base Shader > Effect: Diffuse Shading renders
    without reflections; Luminous Shading renders only glowing surfaces;
    Specular Shading only reflections. Back to Full Shading: as before.
22. **Gradient inputs.** Gradient Input Parameter Locator Incidence,
    XYZ Distance to Locator, Thickness and Bumpless Incident no longer say
    "(stored)" and change the render.
23. **Output Filename / Format.** A Depth output with Output Filename
    "depth/z" and Format OpenEXR (float): saving a render writes
    depth/z.exr next to it. Empty settings keep "Render N (Depth Output)".
24. **Outputs inside a Material Group.** Two cubes with different
    material tags; a Diffuse Color output inside the first tag's Material
    Group: in the render's outputs it shows only the first cube.
