# OpenPBR surface + MaterialX import/export (build spec v1)

Jason approved (F2+F3): MaterialX and OpenPBR first-class and built in. Clean room: public OpenPBR spec, MaterialX spec/stdlib (Apache-2.0) and Cycles/Blender public docs only. Bible rules apply; command names are placeholders under operandi.* until the registry thread names them.

## Stage A (OPM-01..04)

- OPM-01 Material channels gain an internal OpenPBR identity: each channel maps to one OpenPBR parameter or "Operandi-only". UI names unchanged. No look change: current Material->Principled translation stays the reference; add explicit tables Material<->OpenPBR and OpenPBR->Cycles Principled.
- OPM-02 Add channels for missing OpenPBR slots (placeholder UI names fine): coat darkening, fuzz (weight/colour/roughness) if sheen absent, thin film (weight/thickness/IOR), dispersion, transmission depth + scatter colour, thin-walled, diffuse roughness, emission in nits. Defaults = OpenPBR defaults so old files look the same.
- OPM-03 Cycles mapping, one unit-tested module: base_* -> Base Color (x weight), Metallic, Diffuse Roughness; specular_* -> Specular IOR Level, Specular Tint, Roughness, Anisotropic, IOR; coat_* -> Coat Weight/Tint/Roughness/IOR/Normal (coat_darkening<1: Cycles always darkens, log it); fuzz_* -> Sheen Weight/Tint/Roughness; subsurface_* -> Subsurface Weight/Radius/Scale/Anisotropy; transmission_weight/color -> Transmission Weight (+tint), depth/scatter -> auto Volume Absorption with density from depth; dispersion -> Dispersion; thin_film_* -> Thin Film Thickness/IOR (weight 0 => thickness 0); emission_luminance nits + colour -> Emission Color + Strength (one documented constant); geometry_opacity/normal/tangent/coat_normal -> Alpha/Normal/Tangent/Coat Normal; thin_walled -> approximation, flagged.
- OPM-04 Every texture-layer Effect targeting a channel knows its OpenPBR name.

## Stage B (MX-01..08)

- MX-01 Own Rust .mtlx reader/writer. Bundle needed stdlib defs (Apache-2.0, LICENSE in Resources/licenses). Don't link MaterialX C++.
- MX-02 Import open_pbr_surface, standard_surface (via published standard_surface_to_open_pbr graph), gltf_pbr (own table), UsdPreviewSurface.
- MX-03 Simple = each input constant or image node (optionally via texcoord/place2d, multiply by constant, normalmap, separate channel) => Material + Image Map layers exactly like preset browser CD-25, colour space from colorspace attribute.
- MX-04 Not simple => refuse whole material: "needs Node Material (not yet supported): <node>", all or nothing, scene unchanged.
- MX-05 Export: File > Export Material as MaterialX…; preset save optionally writes .mtlx beside .oppreset (gear option). Writes open_pbr_surface + image nodes; mask mixes as mix nodes; else baked to constant with a warning listing losses.
- MX-06 Round trip export->import gives same channels and layers (tested).
- MX-07 Preset browser recognises .mtlx as material entries; .mtlx next to a texture set beats suffix guessing.
- MX-08 Library pack build prefers ambientCG .mtlx.

## Stage C: NOT NOW.

## Acceptance

OPM-T1 old scenes pixel-identical within noise; OPM-T2 OpenPBR example materials render plausibly, gaps logged; MX-T1 ambientCG .mtlx import = same layers as its texture set; MX-T2 export starter-pack Bricks, reimport identical; MX-T3 unsupported node refused, scene unchanged; MX-T4 standard_surface + gltf_pbr import.
