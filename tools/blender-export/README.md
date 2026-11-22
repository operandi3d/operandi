# Blender node graph exporter

Writes every material's node tree in a `.blend` file as plain JSON
(`operandi-node-graph` v1, see `NODE-GRAPH-JSON.md`). Operandi's material
converter (`operandi-preset-convert --graph`) and the Node Material builder
read it. GPL-2.0-or-later, see `LICENSE.md`.

```bash
/Applications/Blender.app/Contents/MacOS/Blender -b Bricks.blend -P tools/blender-export/export.py -- out.json --licence CC0-1.0 --url https://ambientcg.com/a/Bricks113A --credit ambientCG
```

Tested with Blender 5.2.2 on the 20 ambientCG `.blend` files in the test
library. `scripts/build-starter-pack.sh` runs it for the starter pack.
