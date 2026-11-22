# Matcaps

The Matcap display style's built-in matcaps are Blender's
(`release/datafiles/studiolights/matcap/` in the Blender source), CC0 /
public domain; thanks to the Blender community. Each one's diffuse and
specular layers are added, downsampled from 512×512 to 256×256 and stored
as 8-bit sRGB PNG: clay_studio, basic_grey, pearl, resin,
ceramic_lightbulb, metal_carpaint, fullmetal and
check_reflection_horizontal. `src/matcap.rs` lists them; the licence note
is in `docs/licenses/blender-studiolights/license.txt`.
