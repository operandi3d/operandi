# Presets Stage 0: test checklist

QA app: `~/Developer/Operandi-Builds/Operandi presets-stage0 <commit>.app`.
Preset files end in `.oppreset`. The save panel starts in a "Presets"
folder (made the first time you save one).

1. **Material group (PB-T1).** Shading tab: Add Layer > Material Group,
   then with the group selected add a Material and two texture layers
   (e.g. Constant), and change a couple of their settings. Keep the
   Shading tab in front, keep the group selected, then File > Save Item
   Preset… and save it. Expect: status line says "Saved Material preset".
2. **Load it back.** File > New. Shading tab: add a Material Group and
   select it. File > Load Preset… and pick the file. Expect: the group now
   holds the material and both layers, with the same settings. Cmd+Z once
   takes it all away again.
3. **No group selected (refusal).** Click an empty spot so no material
   group is selected (or select the Base Material), then File > Load
   Preset… with the same file. Expect: a message asking you to select a
   material group; nothing changes.
4. **Mesh at the origin (PB-T2).** Make a cube, move it to X = 5 with the
   item transform (Item mode, Move, or the Properties Position X field).
   Right-click it in the Item List > Presets > Save… Expect: a note that it
   will load at the origin. File > New, File > Load Preset…: the cube
   appears at the origin, selected.
5. **Environment (PB-T3).** In a scene with two or three lights placed
   around, File > Save Environment Preset… Open another scene with
   different lights, File > Load Preset… Expect: the old lights are gone
   and the saved ones are there in their places; meshes untouched. One
   Cmd+Z brings the old lights back.
6. **Render settings.** Shading tab: right-click the "Render" row > Save
   Preset… Load it in another scene: the render outputs are replaced and
   the saved render camera is added as an extra camera.
7. **Copied file (PB-T4).** In Finder, copy a saved `.oppreset` somewhere
   else (Desktop), then load it from there, or drag it onto the Operandi
   window. Expect: it loads the same way. (No thumbnails yet in Stage 0.)
8. **Files stay apart.** File > Open… doesn't list presets; Load Preset…
   lists only presets. Old scenes still open as before.

Also there (no need to test separately): Item List right-click Presets >
Save Assembly… (keeps parent/child), Shading right-click a material group
> Presets > Load…, and the `preset.save` / `preset.load` commands.
