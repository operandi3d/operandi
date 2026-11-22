# Node graph JSON (shared format, v1)

The plain description of a material node tree that `export.py` writes and
Operandi reads (`operandi-core/src/node_graph.rs`: the material converter
and the Node Material builder). One file per `.blend`.

```json
{
  "format": "operandi-node-graph", "format_version": 1,
  "source": {"app": "blender", "app_version": "4.2.0", "file": "Bricks01.blend"},
  "licence": {"name": "CC0-1.0", "url": "https://...", "credit": "ambientCG"},
  "materials": [ {
    "name": "...",
    "output": "<name of Material Output node>",
    "nodes": [ {
      "name": "...",
      "type": "<Blender idname e.g. ShaderNodeBsdfPrincipled>",
      "props": { "interpolation": "...", "projection": "...", "extension": "...",
                 "distribution": "...", "vector_type": "...", "blend_type": "...",
                 "data_type": "..." },
      "image": {"path": "relative to the JSON when possible", "colorspace": "<Blender colour space name>", "packed": false},
      "inputs": { "<socket name>": {"type": "VALUE|RGBA|VECTOR|INT|BOOLEAN|SHADER|STRING", "value": "..."} }
    } ],
    "links": [ {"from": ["<node>", "<socket name>", "<identifier, optional, when names repeat>"],
                "to":   ["<node>", "<socket name>", "<identifier optional>"]} ],
    "exposed": [ {"name": "Tint", "to": ["Mix", "B"], "type": "RGBA", "default": [1, 1, 1, 1], "min": null, "max": null} ],
    "groups_used": ["..."]
  } ],
  "groups": [ {"name": "...", "nodes": [], "links": [], "inputs": [], "outputs": []} ]
}
```

## Rules

- `licence` comes from the exporter's `--licence/--url/--credit` arguments,
  else it is `null`.
- `inputs` lists only **unlinked** inputs; linked ones appear in `links`.
- `image` is on Image Texture nodes only (and Environment Texture).
  Packed images are unpacked next to the JSON (`"packed": true`).
- Color Ramp: `props.ramp = {"interpolation": "LINEAR", "elements": [[pos, [r, g, b, a]], ...]}`.
- `exposed` is the group interface when the material is a single group
  node, else `[]`.
- Format changes bump `format_version`.

## Clarifications (v1, no change to the shape)

Written down while building the exporter and converter (2026-11-11):

- **Repeated socket names.** Where a node has several inputs of one name
  (Mix's `A`/`B`/`Factor` per data type, Math's three `Value`s), the
  `inputs` key is the socket **identifier** (`A_Color`, `Factor_Float`,
  `Value_001`), and links give the identifier as their third element.
  Disabled inputs are left out of `inputs`.
- **Value and RGB nodes** write their value under `outputs`, keyed by
  socket name: a Value node `"outputs": {"Value": {"type": "VALUE",
  "value": 0.5}}`, an RGB node `"outputs": {"Color": {"type": "RGBA",
  "value": [r, g, b, a]}}`. Readers fall back to `props.value` /
  `props.color` (early exports).
- **Group interface entries** (`groups[].inputs`, `groups[].outputs`, and
  what feeds a material's `exposed` list) are objects `{"name",
  "identifier" (only when it differs from name), "type", "default"}`.
  `exposed` entries add `"to"`, `"min"` and `"max"` as shown above.
- **Paths.** `image.path` is relative to the JSON only when the picture is
  inside the JSON's folder (or below it); otherwise it is absolute (a
  relative path climbing out with `..` breaks across symlinked folders).
- Reroute nodes are kept (`NodeReroute`, input `Input`, output `Output`);
  frames are left out. Muted and invalid links are left out.
- `props` holds only the settings a node actually has, plus `label` when
  the node has one and `group` (the node tree's name) on group nodes.
- **Socket types are short names everywhere**: node `inputs`, `outputs`
  and group interface entries (so `exposed` too) all write `VALUE`,
  `RGBA`, `VECTOR`, `INT`, `BOOLEAN`, `SHADER` or `STRING`, never a
  Blender idname. The exporter maps `NodeSocketFloat*` → `VALUE`,
  `NodeSocketColor` → `RGBA`, `NodeSocketVector*` → `VECTOR`,
  `NodeSocketInt*` → `INT`, `NodeSocketBool` → `BOOLEAN`,
  `NodeSocketShader` → `SHADER`, `NodeSocketString` → `STRING`.
- **A Group node names its group in `props.group`** (a string, the
  `groups[].name` it uses). Readers may fall back to `props.node_tree`
  for files written another way.
