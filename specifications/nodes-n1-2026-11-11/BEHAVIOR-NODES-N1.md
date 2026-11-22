# Nodes option, phase N1: Node Material layer (build spec)

2026-11-11. Approved by Jason (plan "Blender & open materials in the shader tree", D1-D5).
Clean room: written from Cycles/Blender public docs and our own shader tree spec. Builders never
look at Modo. Bible rules (UIC-xx) apply; cite them. Command names are placeholders under
`operandi.*` until the command registry thread names them.

Licence: node names, sockets and defaults come from Cycles' own node definitions (Apache-2.0).
No Blender UI code is needed for N1. Anything ported from Blender (later: viewport GLSL) goes in
the Blender-derived GPL-2.0-or-later crate with SPDX headers. Jason's rule: nothing that isn't
cleanly GPL v3 compatible.

## Goal
Render imported node-graph materials exactly, inside the existing shader tree, without a node
editor yet. The shader tree stays the default way to make materials.

## NM-01..09 Data model
- NM-01 A **node graph** = nodes (type, name, values per input socket), links (from node.socket
  to node.socket), one Material Output node, and a list of **exposed inputs**.
- NM-02 An exposed input has a display name, the node.socket it feeds, a type (float, colour,
  vector), default, and optional min/max.
- NM-03 Graphs are saved as versioned plain text inside the scene file (own section, own version
  number). Unknown node types on load are kept as-is and flagged, never dropped.
- NM-04 A graph may live in the shader tree's **Nodes** section as a shared node group; layers
  reference it by id. Editing a shared group's exposed defaults affects every layer using it
  unless the layer overrides that value.
- NM-05 Input format for imports = the JSON written by `tools/blender-export/` (step 1 build).
  The importer maps Blender node identifiers to our node types; unmapped node types = whole
  material refused with a message naming the node (all or nothing).

## NM-10..19 The Node Material layer in the shader tree
- NM-10 New layer type "Node Material". Lives inside a Material Group like other layers; the
  group's polygon tag, item/part masks, enable and the group's own mask decide where it shows.
- NM-11 It outputs a finished surface (a closure), like a Material + Shader in one. It ignores
  channel layers placed **below** it in the same group (it replaces them where its opacity is 100%).
- NM-12 Layer Opacity blends it over the result below with a Cycles Mix Shader (factor =
  opacity x layer mask). Blend mode: Normal only in N1; other modes greyed with a tooltip.
- NM-13 Texture layers **above** it can't reach inside, except exposed inputs: each exposed
  input appears as an extra Effect choice (listed under a "Node Inputs" heading in the Effect
  menu). A layer targeting it drives that socket the same way it drives a normal channel.
- NM-14 Properties panel for the layer: name, graph (pick from Nodes section), then one control
  per exposed input (shared control types; colour swatch, float slider, vector fields).
- NM-15 Double-click the layer (or "Show Graph" in its right-click menu) opens a **read-only
  graph view** in its own OS window (Operandi::palette_window, Jason's floating-panel rule).
  Pan/zoom, select a node to see its values. No editing in N1.
- NM-16 Each edit to exposed values or opacity is one undo step.
- NM-17 Saving a group holding a Node Material as a material .oppreset embeds the graph.
  Preset browser shows a "Nodes" badge on such presets (plan section 4).

## NM-20..29 Node set for N1 (25)
Shaders: Principled BSDF, Mix Shader, Emission, Transparent BSDF, Glass BSDF.
Textures: Image Texture, Noise, Voronoi, Wave, Gradient.
Input: Texture Coordinate, Mapping, Layer Weight, Fresnel, RGB, Value.
Colour/convert: Mix Color (all blend modes), Color Ramp, Math, Separate Color, Combine Color, Invert.
Vector: Normal Map, Bump, Displacement.
Plus Group Input / Group Output for shared groups, and Material Output.
- NM-21 Image Texture: colour space per image (sRGB / Non-Color), images linked by path and shown
  in Clips like other images; missing file = shared missing-file look (CD-27).
- NM-22 Displacement output: honoured the same way the shader tree's displacement is today
  (bump-only or true, whatever the tree currently does).

## NM-30..39 Rendering
- NM-30 Widen the Cycles C ABI from the current flattened ops (CONST, IMAGE_COLOR, IMAGE_ALPHA,
  LUMA, MIX) to a general node list: node type + input values + links. Bump the ABI version.
  Keep the old op path working for layer-only materials (no look change for existing scenes;
  existing render tests must still pass).
- NM-31 A context that mixes layers and a Node Material compiles to one Cycles shader: layer
  stack -> Principled as today, then Mix Shader with the node graph closure.
- NM-32 Change to an exposed value updates only that shader (no full scene rebuild).

## NM-40..44 Viewport (N1 only)
- NM-40 Until the viewport-materials work can draw graphs, the viewport shows an approximation:
  a small background bake (e.g. 256 px) of base colour + roughness from Cycles, on UVs, cached
  by graph hash. While baking: the graph's Principled base colour constant if any, else mid grey.

## Out of scope (N2+)
Node editing (add/wire/delete), other blend modes for node layers, convert layers <-> nodes,
Bake to textures command, MaterialX import, nodes beyond the 25.

## Acceptance
- NM-T1 Import a graph using Noise -> Color Ramp -> Principled Base Color: renders in Cycles
  matching a reference render of the same graph made with Cycles (pixel diff within noise).
- NM-T2 Node Material at 50% opacity over a red layered material: half-mix visible; one undo
  restores 100%.
- NM-T3 Exposed input "Tint" driven by a Gradient layer above: tint follows the gradient.
- NM-T4 Graph with an unsupported node: refused, message names the node, scene unchanged.
- NM-T5 Save, quit, reopen: graph, exposed values, overrides intact. Old scenes open unchanged.
- NM-T6 Read-only graph view opens in its own window, survives relaunch position (UIC-73).
