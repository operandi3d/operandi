# TextureCraft pieces: GUI checklist

Branch `claude/project-thread-rccgyy` (2026-11-17). New: FBX export, 3MF
import/export, Texture > Atlas Unwrap…, Bake Curvature…, Bake Thickness…,
Pad Texture to UVs…. Painting code is parked, not in the app.

Start the QA app with `OPERANDI_CYCLES=1`. Use a scratch folder for files.

## Atlas Unwrap

1. Create a Cube (or open any mesh). Texture > Atlas Unwrap…
2. The dialog shows UV Map, Max Angle (60 °), Gaps (4 px), Texture Size
   (1024 px), OK over Cancel on the right (UIC-81..99). Press Return.
3. View > UV View (or Texture > Open UV Editor): six square-ish islands, no
   overlaps. Cmd+Z puts the old UVs back in one step.
4. Also on the UV toolbox tab: "Atlas Unwrap..." under Pack UVs.
5. Type `operandi.uv.atlas angle:30deg` in the command line: more islands.

## Bakes

1. With the unwrapped cube selected: Texture > Bake Curvature… → OK → save
   as `cube_curvature.png`. Status: "Baked Curvature 1024×1024 to …".
2. Open the PNG (Finder preview): grey faces, light edges, grey background
   filled around the islands (no hard black borders).
3. Texture > Bake Thickness… on a thin plate next to a thick box (one mesh):
   the thin part is dark, the thick part light.
4. Try Size 4096 once: it takes a few seconds and the window pauses while
   it bakes (known, see below).
5. Typed: `operandi.bake.curvature filename:/tmp/c.png size:512`.

## Pad Texture to UVs

1. Paint or draw any PNG whose colours sit only inside the islands (the
   curvature bake with Padding 0 and Fill Background off works).
2. Texture > Pad Texture to UVs… → OK → pick the PNG. A copy named
   `<name>_padded.png` appears beside it with colour grown 16 px past every
   island edge; the original is untouched.

## FBX and 3MF

1. File > Export ▸ FBX (.fbx). Re-import it with File > Import: same shape,
   materials and UVs.
2. Open the FBX in another app you have (Blender, Unity, Maya, an online
   viewer): it should open at the right size (metres), upright, with
   materials. **This is the main unknown: please note which apps you
   tried.**
3. File > Export ▸ 3D Manufacturing (.3mf), then open it in a slicer
   (PrusaSlicer, Bambu Studio, Cura): right size in mm, upright.
4. Import a 3MF from a slicer or Printables: parts land as separate items.

## Known limits

- Bakes run on the main thread: big sizes freeze the window until done.
- Bakes use the mesh as it is (no subdivision, no high-poly source, no
  cage); only the 0–1 UV tile is baked (no UDIMs).
- FBX export has no hierarchy, cameras, lights or animation.
