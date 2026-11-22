# Nodes N1 GUI check (Jason or Codex)

Use the QA app named in the thread; fixture `fixtures/node-graphs/noise_ramp.json`.

1. Shading tab, Add Layer > Import Node Material..., pick the fixture. A
   "Noise Ramp" layer (letter N) appears, selected; Nodes shows "Noise Ramp".
2. Render (Preview or F9): the mesh shows blue-to-orange noise.
3. Drag the layer's % to 50: the render mixes it with the material below.
   Undo once: back to 100 %.
4. Properties: change Tint (swatch) and Roughness; a dot marks the layer's
   own value. Reset to Default on a row follows the graph again.
5. Click "Noise Ramp" under Nodes: change its Tint default; the layer
   follows unless it has its own value.
6. Add a Constant layer above the Node Material, Effect > Node Inputs >
   Tint, colour red: the render tints red.
7. Blend Mode on the Node Material: every mode but Normal is greyed with
   a reason.
8. Double-click the layer (or right-click > Show Graph): the graph opens
   in the Node View panel (docked under the views). Drag pans, scroll/pinch
   zooms, clicking a node lists its values. Quit, relaunch: the panel is
   where you left it (it's part of the layout).
9. Save, quit, reopen the scene: layer, graph and values are the same.
10. Import a JSON with an unsupported node (e.g. edit "ShaderNodeTexNoise"
    to "ShaderNodeTexBrick"): status names the node; nothing changes.
11. (Node editor slice 1) File > Import MaterialX…
    `fixtures/materialx-examples/StandardSurface/standard_surface_marble_solid.mtlx`:
    it lands as a graph layer. Right-click it > Show Graph: the window
    shows its MaterialX nodes (fractal3d, mix…) and links; clicking one
    lists its values and what feeds it.
12. Right-click a Material Group > Show as Nodes: the window shows that
    material's graph (operandi_* nodes into operandi_surface). Change a
    layer's colour or add an Image Map: the window follows after the edit.
    Close it, delete the group, reopen: nothing stale shows.
