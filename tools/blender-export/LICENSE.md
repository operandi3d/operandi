# Licence of this folder

Everything in `tools/blender-export/` is licensed **GPL-2.0-or-later**
(`LICENSES/GPL-2.0-or-later.txt`), not Operandi's GPL-3.0-or-later, because
`export.py` imports Blender's `bpy` module and runs inside Blender.

- This folder is a separate tool. Nothing here is compiled into, linked
  into or shipped inside the Operandi app; Operandi only reads the plain
  JSON files the tool writes (format in `NODE-GRAPH-JSON.md`).
- Each source file carries an SPDX header. Keep it when you change a file.
- `samples/` holds exporter output made from CC0 content (ambientCG
  Metal049A, https://ambientcg.com/a/Metal049A, CC0 1.0).
- `samples/value-rgb-group.json` is exporter output from a scene made for
  the test (Value and RGB nodes, a node group interface); no third-party
  content.
