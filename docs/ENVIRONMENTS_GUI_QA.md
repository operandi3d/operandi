# Environments: GUI QA checklist (branches environments, env-not-in-items)

Run the QA build with `OPERANDI_CYCLES=1`. Specs:
development/project-files/environments-2026-11-09 and
development/project-files/ENVIRONMENT-PLACEMENT.md in the worktree.

1. **New scene.** File > New. Items list: Mesh, Camera, Directional Light
   and **no Environment** (try Filter > Sort by Type, Flatten, the search
   funnel with "env", and Sel: still none). Shader Tree tab: the sections
   read Render, Library, Nodes, Lights, Environments, Bake Items, FX.
   Environments has an arrow but starts **closed**, has no icon and no eye.
2. **Header.** Click the Environments header: it only opens or closes; the
   selection doesn't change. Double-click it: no rename. Try dragging it:
   it doesn't move.
3. **Rows.** Open it: "Environment" with the Environment icon, an eye and
   its own arrow; under it "Environment Material" (M). Click Environment:
   Properties shows Intensity, the four Visible to … switches and the LPE
   label only: **no Item Transform, no Rotation Order, no Display or
   Assembly tabs**. Click Environment Material: Name, Layer block,
   Environment Type, four colours, two exponents, then the collapsed
   Physically-based Daylight and Environment Fog groups. Change the type:
   rows that don't apply grey out.
4. **Render.** Render (or Preview): the background is the light-grey
   gradient (0.90 overhead, 0.34 at the horizon). Set Zenith red: the top
   of a camera looking up turns red. Intensity 2 doubles it.
5. **Create.** Item > Create Environment (the Item menu's first entry):
   "Environment (2)" appears at the **top** of the section with
   "Environment Material (2)", is selected, and the section opens. The
   Items list still shows no environment. It shows in renders.
6. **Selection.** With three environments: click the top one, Shift+click
   the bottom one: all three are selected. Cmd+click the middle one: it
   drops out. The Items list shows nothing new either way.
7. **Eye.** Click the top environment's eye: it dims and renders show the
   next one down; Edit > Undo brings it back. Turn its Visible to Camera
   off instead: the next one shows behind objects, but the top one still
   lights the scene.
8. **Right-click an Environment row.** Rename…, Editor Color, Properties,
   divider, Duplicate, Delete, divider, Presets (greyed; hover says why).
   Rename… edits the name in place. Editor Color tints the row. Duplicate
   copies it with its layers (new "Environment Material (n)"). Delete
   removes it; delete every one: the section loses its arrow and renders
   go black. Undo brings them back.
9. **Layers.** Select an environment, Add Layer > Processing > Constant: a
   Constant appears above its material. Colour green, Opacity 50 %: the
   background blends halfway.
10. **Image.** Type `clip.addStill <path to an HDR or EXR lat-long image>`,
    then Add Layer > Image Map with the environment selected; pick the
    image. Render: the sky shows the image; select its Texture Locator
    (button in Properties) and rotate it about Y: the image turns. The
    environment itself can't be rotated.
11. **Daylight.** Environment Type Physically-based Daylight, Sun Light =
    Directional Light. Render: a blue sky with the sun where the light
    comes from. Rotate the light: the sun moves.
12. **Viewport.** Viewport Properties > Background: Environment. The view
    background shows the gradient with a hard horizon; orbit and check it
    stays smooth and quick.
13. **Typed commands (UIC-164).** With the environment selected type
    `item.channel intensity 0.5`, then `item.channel intensity ?`: it
    answers 0.5, and Edit > Undo puts it back in one step.
    `item.channel rotOrder ZXY` says the environment has no such channel.
    With only the environment selected, the Move tool refuses ("Environments
    have no transform…").
14. **Save / reopen.** Save, close, reopen: environments, layers and
    settings come back. Open a scene saved before environments: it gets the
    default environment. A scene from an earlier environments build where
    the environment was rotated opens with the sky upright, and nothing
    that was parented under it moves.
15. **Texture Locator (env-texture-locator).** Load an HDRI, select its
    image layer under the environment, open the Texture Locator sub-tab:
    Rotation Order ZXY, Projection Type Spherical, Projection Axis Y.
    Text in the picture reads the right way round (not mirrored) in the
    viewport and in a render.
    Drag Rotation Y to 90°: the sky turns live in the viewport (Background:
    Environment) and one Edit > Undo puts it back. Render (File > Render
    Viewport): the picture faces the same way as the viewport. Rotation X
    tilts the horizon; Projection Axis Z lays the picture's pole along +Z.
    Position and Size change nothing on the sky; Auto Size is greyed with a
    reason. Save, reopen: the locator comes back as set. Load a mirror-ball
    photo and set Projection Type to Light Probe: the ball unwraps to a
    full sky, the same way in the viewport and in a render.
16. **XYZ order (v44).** Open a scene saved before this build with an item
    turned about two axes in XYZ: it sits exactly where it did, and its
    Rotation Order now reads ZYX.
